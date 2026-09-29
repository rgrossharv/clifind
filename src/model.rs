//! Optional local-model integration. It is deliberately opt-in: ordinary
//! searches are deterministic and do not load a model or start a process.

use crate::engine::ResultItem;
use std::env;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MAX_PARAMETERS: u64 = 1_000_000_000;
const MAX_OUTPUT_BYTES: usize = 600;
const TIMEOUT: Duration = Duration::from_secs(3);

/// Return the number of tensor elements recorded in a GGUF model. This
/// measures the stored model directly rather than trusting a filename.
pub fn gguf_parameter_count(path: &Path) -> Result<u64, String> {
    let file = File::open(path)
        .map_err(|error| format!("cannot open model {}: {error}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut magic = [0_u8; 4];
    reader.read_exact(&mut magic).map_err(io_error)?;
    if &magic != b"GGUF" {
        return Err("the local model must be a GGUF file".into());
    }
    let version = read_u32(&mut reader)?;
    if !(2..=3).contains(&version) {
        return Err(format!(
            "unsupported GGUF version {version}; expected version 2 or 3"
        ));
    }
    let tensor_count = read_u64(&mut reader)?;
    let metadata_count = read_u64(&mut reader)?;
    if tensor_count > 1_000_000 || metadata_count > 1_000_000 {
        return Err("GGUF header contains an unreasonable item count".into());
    }
    for _ in 0..metadata_count {
        skip_string(&mut reader)?;
        skip_metadata_value(&mut reader)?;
    }

    let mut total = 0_u64;
    for _ in 0..tensor_count {
        skip_string(&mut reader)?;
        let dimensions = read_u32(&mut reader)?;
        if dimensions == 0 || dimensions > 8 {
            return Err("GGUF tensor has an invalid dimension count".into());
        }
        let mut elements = 1_u64;
        for _ in 0..dimensions {
            elements = elements
                .checked_mul(read_u64(&mut reader)?)
                .ok_or("GGUF tensor size overflow")?;
        }
        total = total
            .checked_add(elements)
            .ok_or("GGUF parameter count overflow")?;
        let _type = read_u32(&mut reader)?;
        let _offset = read_u64(&mut reader)?;
    }
    Ok(total)
}

pub fn explain(query: &str, results: &[ResultItem]) -> Result<String, String> {
    let model = env::var_os("CLIFIND_LLM_MODEL")
        .ok_or("--explain needs CLIFIND_LLM_MODEL pointing to a local GGUF model")?;
    let binary = env::var_os("CLIFIND_LLM_BIN")
        .ok_or("--explain needs CLIFIND_LLM_BIN pointing to a local llama.cpp-compatible runner")?;
    let parameters = gguf_parameter_count(Path::new(&model))?;
    if parameters > MAX_PARAMETERS {
        return Err(format!(
            "refusing model with {parameters} parameters; clifind's hard limit is {MAX_PARAMETERS}"
        ));
    }

    let options = results
        .iter()
        .take(4)
        .map(|item| {
            format!(
                "{} — {}",
                item.candidate.command, item.candidate.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let prompt = format!(
        "In one sentence (under 40 words), explain which terminal command best addresses this request. Do not invent commands or instructions. Request: {query}\nOptions:\n{options}"
    );
    let mut child = Command::new(binary)
        .arg("-m")
        .arg(model)
        .arg("-p")
        .arg(prompt)
        .arg("-n")
        .arg("80")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("could not start the configured local model runner: {error}"))?;
    let started = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|error| format!("could not poll local model runner: {error}"))?
            .is_some()
        {
            let output = child
                .wait_with_output()
                .map_err(|error| format!("could not read local model output: {error}"))?;
            if !output.status.success() {
                return Err("the local model runner exited unsuccessfully".into());
            }
            let text = String::from_utf8_lossy(&output.stdout);
            let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
            return Ok(compact.chars().take(MAX_OUTPUT_BYTES).collect());
        }
        if started.elapsed() >= TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err("the local model runner exceeded clifind's three-second limit".into());
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn read_u32(reader: &mut impl Read) -> Result<u32, String> {
    let mut bytes = [0_u8; 4];
    reader.read_exact(&mut bytes).map_err(io_error)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(reader: &mut impl Read) -> Result<u64, String> {
    let mut bytes = [0_u8; 8];
    reader.read_exact(&mut bytes).map_err(io_error)?;
    Ok(u64::from_le_bytes(bytes))
}

fn skip_string(reader: &mut impl Read) -> Result<(), String> {
    let length = read_u64(reader)?;
    if length > 64 * 1024 * 1024 {
        return Err("GGUF string is unreasonably large".into());
    }
    copy_exact(reader, length)
}

fn skip_metadata_value(reader: &mut impl Read) -> Result<(), String> {
    let value_type = read_u32(reader)?;
    skip_value(reader, value_type)
}

fn skip_value(reader: &mut impl Read, value_type: u32) -> Result<(), String> {
    match value_type {
        0 | 1 | 7 => copy_exact(reader, 1),
        2 | 3 => copy_exact(reader, 2),
        4..=6 => copy_exact(reader, 4),
        10..=12 => copy_exact(reader, 8),
        8 => skip_string(reader),
        9 => {
            let item_type = read_u32(reader)?;
            let count = read_u64(reader)?;
            if count > 10_000_000 {
                return Err("GGUF array is unreasonably large".into());
            }
            for _ in 0..count {
                skip_value(reader, item_type)?;
            }
            Ok(())
        }
        _ => Err(format!("unsupported GGUF metadata type {value_type}")),
    }
}

fn copy_exact(reader: &mut impl Read, mut length: u64) -> Result<(), String> {
    let mut buffer = [0_u8; 8192];
    while length > 0 {
        let chunk = usize::try_from(length.min(buffer.len() as u64)).unwrap();
        reader.read_exact(&mut buffer[..chunk]).map_err(io_error)?;
        length -= chunk as u64;
    }
    Ok(())
}

fn io_error(error: io::Error) -> String {
    format!("invalid GGUF file: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture(tensor_dimensions: &[Vec<u64>]) -> Vec<u8> {
        let mut bytes = b"GGUF".to_vec();
        bytes.extend(3_u32.to_le_bytes());
        bytes.extend((tensor_dimensions.len() as u64).to_le_bytes());
        bytes.extend(0_u64.to_le_bytes());
        for dimensions in tensor_dimensions {
            bytes.extend(1_u64.to_le_bytes());
            bytes.push(b'x');
            bytes.extend((dimensions.len() as u32).to_le_bytes());
            for dimension in dimensions {
                bytes.extend(dimension.to_le_bytes());
            }
            bytes.extend(0_u32.to_le_bytes());
            bytes.extend(0_u64.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn counts_parameters_from_tensor_shapes() {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("clifind-model-{id}.gguf"));
        fs::write(&path, fixture(&[vec![100, 10], vec![5, 20]])).unwrap();
        assert_eq!(gguf_parameter_count(&path).unwrap(), 1_100);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_non_gguf_input() {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("clifind-model-{id}.bad"));
        fs::write(&path, b"not a model").unwrap();
        assert!(gguf_parameter_count(&path).is_err());
        fs::remove_file(path).unwrap();
    }
}

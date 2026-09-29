use crate::engine::{Candidate, Platform, Source};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_PATH_COMMANDS: usize = 5_000;
const MAX_SCRIPT_BYTES: u64 = 256 * 1024;

/// Discover user-installed executables, simple shell aliases, and an optional
/// declarative config file. Discovery reads files only; it never executes a
/// local command while building the index.
pub fn discover() -> Vec<Candidate> {
    let mut candidates = discover_path_commands();
    candidates.extend(discover_aliases());
    candidates.extend(discover_config());
    candidates
}

pub fn discover_path_commands() -> Vec<Candidate> {
    let mut directories: Vec<PathBuf> = env::var_os("PATH")
        .map(|paths| env::split_paths(&paths).collect())
        .unwrap_or_default();

    // Common user command locations are useful in non-login shells too. A
    // command outside PATH remains labelled with its source and is never run.
    if let Some(home) = home_dir() {
        directories.push(home.join(".local/bin"));
        directories.push(home.join("bin"));
    }

    let mut seen_paths = HashSet::new();
    let mut seen_names = HashSet::new();
    let mut results = Vec::new();
    for directory in directories {
        let canonical = directory.canonicalize().unwrap_or(directory);
        if !seen_paths.insert(canonical.clone()) {
            continue;
        }
        let Ok(entries) = fs::read_dir(&canonical) else {
            continue;
        };
        for entry in entries.flatten() {
            if results.len() >= MAX_PATH_COMMANDS {
                return results;
            }
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !seen_names.insert(name.to_string())
                || !is_executable_file(&path)
                || is_system_directory(&canonical)
            {
                continue;
            }
            results.push(candidate_from_path(name, &path));
        }
    }
    results
}

pub fn discover_aliases() -> Vec<Candidate> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    let files = [
        home.join(".bashrc"),
        home.join(".bash_profile"),
        home.join(".zshrc"),
        home.join(".config/fish/config.fish"),
    ];
    let mut names = HashSet::new();
    let mut results = Vec::new();
    for file in files {
        let Ok(contents) = fs::read_to_string(&file) else {
            continue;
        };
        for line in contents.lines() {
            let trimmed = line.trim();
            let parsed = parse_alias(trimmed).or_else(|| parse_fish_alias(trimmed));
            let Some((name, expansion)) = parsed else {
                continue;
            };
            if name.is_empty() || !names.insert(name.to_string()) {
                continue;
            }
            results.push(Candidate {
                command: name.to_string(),
                description: format!("Shell alias expanding to: {expansion}"),
                keywords: format!("{name} {expansion} custom alias"),
                platforms: Platform::Both,
                source: Source::Alias(file.display().to_string()),
            });
        }
    }
    results
}

pub fn discover_config() -> Vec<Candidate> {
    let Some(path) = config_path() else {
        return Vec::new();
    };
    let Ok(contents) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let mut results = Vec::new();
    for (index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.splitn(3, '\t').collect();
        if fields.len() < 2 || fields[0].trim().is_empty() || fields[1].trim().is_empty() {
            eprintln!(
                "clifind: ignoring malformed custom command at {}:{}",
                path.display(),
                index + 1
            );
            continue;
        }
        results.push(Candidate {
            command: fields[0].trim().to_string(),
            description: fields[1].trim().to_string(),
            keywords: fields
                .get(2)
                .map(|value| value.trim())
                .unwrap_or("")
                .to_string(),
            platforms: Platform::Both,
            source: Source::Config(path.display().to_string()),
        });
    }
    results
}

fn candidate_from_path(name: &str, path: &Path) -> Candidate {
    let hint = script_hint(path);
    let path_display = path.display().to_string();
    let description =
        hint.unwrap_or_else(|| format!("Local executable discovered at {path_display}."));
    Candidate {
        command: name.to_string(),
        keywords: format!(
            "{name} {} {} custom local executable",
            path_display, description
        ),
        description,
        platforms: Platform::Both,
        source: Source::Path(path_display),
    }
}

fn script_hint(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > MAX_SCRIPT_BYTES || metadata.len() == 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(path)
        .ok()?
        .take(MAX_SCRIPT_BYTES)
        .read_to_end(&mut bytes)
        .ok()?;
    let contents = std::str::from_utf8(&bytes).ok()?;
    let mut comments = Vec::new();
    for line in contents.lines().take(24) {
        let line = line.trim();
        if let Some(comment) = line.strip_prefix('#') {
            let comment = comment.trim();
            if !comment.starts_with('!') && !comment.is_empty() {
                comments.push(comment);
            }
        } else if !line.is_empty() && !line.starts_with("//") {
            break;
        }
    }
    let hint = comments.join(" ");
    let hint: String = hint.chars().take(280).collect();
    (!hint.is_empty()).then_some(hint)
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// OS command folders contain thousands of implementation scripts. They are
/// already represented by the curated catalogue where relevant, and indexing
/// their comments makes ordinary results noisy. `/usr/local` is intentionally
/// not included: it is a conventional location for user-installed tools.
fn is_system_directory(path: &Path) -> bool {
    matches!(
        path.to_str(),
        Some("/bin") | Some("/sbin") | Some("/usr/bin") | Some("/usr/sbin") | Some("/usr/libexec")
    )
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

fn parse_alias(line: &str) -> Option<(&str, String)> {
    let expression = line.strip_prefix("alias ")?.trim();
    let (name, value) = expression.split_once('=')?;
    let name = name.trim();
    let value = value.trim().trim_matches('\'').trim_matches('"');
    (!name.is_empty() && !value.is_empty()).then(|| (name, value.to_string()))
}

fn parse_fish_alias(line: &str) -> Option<(&str, String)> {
    let expression = line.strip_prefix("alias ")?.trim();
    if expression.contains('=') {
        return None;
    }
    let (name, value) = expression.split_once(' ')?;
    let value = value.trim().trim_matches('\'').trim_matches('"');
    (!name.is_empty() && !value.is_empty()).then(|| (name, value.to_string()))
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

fn config_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("CLIFIND_COMMANDS_FILE") {
        return Some(PathBuf::from(path));
    }
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".config")))?;
    Some(base.join("clifind/commands.tsv"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_bash_and_fish_aliases() {
        assert_eq!(
            parse_alias("alias ll='ls -lah'"),
            Some(("ll", "ls -lah".to_string()))
        );
        assert_eq!(
            parse_fish_alias("alias gs git status"),
            Some(("gs", "git status".to_string()))
        );
        assert_eq!(parse_alias("export PATH=/bin"), None);
    }

    #[test]
    fn turns_a_local_script_comment_into_searchable_description() {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("clifind-minetoggle-{id}"));
        fs::write(
            &path,
            "#!/bin/sh\n# Remote-friendly on/off control for the configured miner.\necho noop\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).unwrap();
        }
        let candidate = candidate_from_path("minetoggle", &path);
        assert!(candidate.description.contains("miner"));
        assert!(candidate.keywords.contains("minetoggle"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn does_not_treat_binary_data_as_a_script_hint() {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("clifind-binary-{id}"));
        fs::write(&path, [0_u8, 159, 146, 150]).unwrap();
        assert_eq!(script_hint(&path), None);
        fs::remove_file(path).unwrap();
    }
}

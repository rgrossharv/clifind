mod catalog;
mod discovery;
mod engine;
mod model;

use engine::{Platform, ResultItem};
use std::env;
use std::io::{self, IsTerminal};
use std::process;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
struct Options {
    query: String,
    limit: usize,
    json: bool,
    custom: bool,
    all_platforms: bool,
    explain: bool,
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(message) => {
            eprintln!("clifind: {message}");
            process::exit(2);
        }
    }
}

fn run() -> Result<(), String> {
    let options = parse_args(env::args().skip(1).collect())?;
    let mut candidates = catalog::builtins();
    if options.custom {
        candidates.extend(discovery::discover());
    }
    let results = engine::search(
        &options.query,
        &candidates,
        Platform::current(),
        options.limit,
        options.all_platforms,
    );

    if options.json {
        print_json(&options.query, &results);
    } else {
        print_human(&results);
    }
    if options.explain && !results.is_empty() {
        match model::explain(&options.query, &results) {
            Ok(explanation) if !explanation.is_empty() => println!("\nLocal model: {explanation}"),
            Ok(_) => eprintln!("clifind: the local model returned no explanation"),
            Err(error) => eprintln!("clifind: local explanation unavailable: {error}"),
        }
    }
    Ok(())
}

fn parse_args(args: Vec<String>) -> Result<Options, String> {
    let mut query = Vec::new();
    let mut limit = 4_usize;
    let mut json = false;
    let mut custom = true;
    let mut all_platforms = false;
    let mut explain = false;
    let mut literal_query = false;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if literal_query {
            query.push(arg.clone());
        } else {
            match arg.as_str() {
                "-h" | "--help" => {
                    print_help();
                    process::exit(0);
                }
                "-V" | "--version" => {
                    println!("clifind {VERSION}");
                    process::exit(0);
                }
                "--json" => json = true,
                "--no-custom" => custom = false,
                "--all-platforms" => all_platforms = true,
                "--explain" => explain = true,
                "--" => literal_query = true,
                "-n" | "--limit" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or("--limit requires a positive number")?;
                    limit = value
                        .parse()
                        .map_err(|_| format!("invalid result limit: {value:?}"))?;
                    if limit == 0 {
                        return Err("--limit must be at least 1".into());
                    }
                }
                _ if arg.starts_with("--limit=") => {
                    let value = arg.trim_start_matches("--limit=");
                    limit = value
                        .parse()
                        .map_err(|_| format!("invalid result limit: {value:?}"))?;
                    if limit == 0 {
                        return Err("--limit must be at least 1".into());
                    }
                }
                _ if arg.starts_with('-') => {
                    return Err(format!("unknown option: {arg}\nTry `clifind --help`."))
                }
                _ => query.push(arg.clone()),
            }
        }
        index += 1;
    }
    if query.is_empty() {
        return Err(
            "tell me what you want to do, for example: clifind turn off my computer".into(),
        );
    }
    Ok(Options {
        query: query.join(" "),
        limit,
        json,
        custom,
        all_platforms,
        explain,
    })
}

fn print_human(results: &[ResultItem]) {
    if results.is_empty() {
        println!("No commands found. Try different words, `--all-platforms`, or add ~/.config/clifind/commands.tsv.");
        return;
    }
    let color = io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none();
    for (index, item) in results.iter().enumerate() {
        let command = if color {
            format!("\x1b[1;36m{}\x1b[0m", item.candidate.command)
        } else {
            item.candidate.command.clone()
        };
        let source = if color {
            format!(
                "\x1b[2m[{}; {}]\x1b[0m",
                item.candidate.platforms.label(),
                item.candidate.source.label()
            )
        } else {
            format!(
                "[{}; {}]",
                item.candidate.platforms.label(),
                item.candidate.source.label()
            )
        };
        println!(
            "{}. {} — {} {}",
            index + 1,
            command,
            item.candidate.description,
            source
        );
    }
    println!("\nSuggestions only — clifind never executes a result.");
}

fn print_json(query: &str, results: &[ResultItem]) {
    print!("{{\"query\":\"{}\",\"results\":[", escape_json(query));
    for (index, item) in results.iter().enumerate() {
        if index > 0 {
            print!(",");
        }
        print!(
            "{{\"command\":\"{}\",\"description\":\"{}\",\"platform\":\"{}\",\"source\":\"{}\",\"score\":{}}}",
            escape_json(&item.candidate.command),
            escape_json(&item.candidate.description),
            item.candidate.platforms.label(),
            escape_json(&item.candidate.source.label()),
            item.score,
        );
    }
    println!("]}}");
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            control if control.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", control as u32))
            }
            character => escaped.push(character),
        }
    }
    escaped
}

fn print_help() {
    println!(
        "\
clifind {VERSION} — find a terminal command from a plain-English request

USAGE:
  clifind [OPTIONS] <WHAT YOU WANT TO DO>

OPTIONS:
  -n, --limit <N>    Number of results (default: 4)
      --no-custom     Do not inspect local commands, aliases, or config
      --all-platforms Include macOS and Linux commands, not only this OS
      --json          Print machine-readable JSON
      --explain       Ask an opt-in, sub-1B local GGUF model for a summary
  -V, --version       Print the version
  -h, --help          Print this help

EXAMPLES:
  clifind turn off my computer
  clifind find text in files
  clifind --json show listening ports
  clifind turn mining on

CUSTOM COMMANDS:
  clifind reads executable commands in PATH (plus ~/.local/bin and ~/bin),
  simple aliases from common shell configs, and ~/.config/clifind/commands.tsv.
  Config rows are: command<TAB>description<TAB>optional keywords.

LOCAL MODEL:
  Normal searches never load a model. --explain requires CLIFIND_LLM_MODEL
  (a local GGUF file) and CLIFIND_LLM_BIN (a local llama.cpp-compatible
  runner). clifind counts GGUF tensor elements and refuses any model above
  1,000,000,000 parameters; the runner is limited to three seconds.
"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_options_and_query() {
        let options = parse_args(vec![
            "--json".into(),
            "-n".into(),
            "2".into(),
            "find".into(),
            "files".into(),
        ])
        .unwrap();
        assert!(options.json);
        assert_eq!(options.limit, 2);
        assert_eq!(options.query, "find files");
    }

    #[test]
    fn rejects_bad_options() {
        assert!(parse_args(vec!["--nope".into(), "files".into()]).is_err());
        assert!(parse_args(vec!["--limit".into(), "0".into(), "files".into()]).is_err());
        assert!(parse_args(Vec::new()).is_err());
    }

    #[test]
    fn produces_valid_escaped_json_strings() {
        assert_eq!(escape_json("a\"\\\n"), "a\\\"\\\\\\n");
    }

    #[test]
    fn local_minetoggle_style_command_is_ranked_for_mining_request() {
        let custom = engine::Candidate {
            command: "minetoggle".into(),
            description: "Remote-friendly on/off control for the configured miner.".into(),
            keywords: "minetoggle miner mining on off custom".into(),
            platforms: Platform::Both,
            source: engine::Source::Path("/home/example/.local/bin/minetoggle".into()),
        };
        let nearby_custom = engine::Candidate {
            command: "mineskip".into(),
            description: "Skip the next scheduled mining run.".into(),
            keywords: "miner mining kryptex tari skip custom".into(),
            platforms: Platform::Both,
            source: engine::Source::Path("/home/example/.local/bin/mineskip".into()),
        };
        let result = engine::search(
            "turn mining on",
            &[nearby_custom, custom],
            Platform::Linux,
            4,
            false,
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].candidate.command, "minetoggle", "{result:#?}");
    }
}

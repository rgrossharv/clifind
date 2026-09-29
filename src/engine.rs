#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Platform {
    Linux,
    Mac,
    Both,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Mac
        } else {
            Self::Linux
        }
    }

    pub fn matches(self, current: Self) -> bool {
        self == Self::Both || self == current
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Mac => "macos",
            Self::Both => "macos/linux",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Source {
    Builtin,
    Path(String),
    Alias(String),
    Config(String),
}

impl Source {
    pub fn label(&self) -> String {
        match self {
            Self::Builtin => "built-in".to_string(),
            Self::Path(path) => format!("local: {}", path),
            Self::Alias(path) => format!("shell alias: {}", path),
            Self::Config(path) => format!("config: {}", path),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Candidate {
    pub command: String,
    pub description: String,
    pub keywords: String,
    pub platforms: Platform,
    pub source: Source,
}

impl Candidate {
    pub fn builtin(command: &str, description: &str, keywords: &str, platforms: Platform) -> Self {
        Self {
            command: command.into(),
            description: description.into(),
            keywords: keywords.into(),
            platforms,
            source: Source::Builtin,
        }
    }

    pub fn searchable_text(&self) -> String {
        format!("{} {} {}", self.command, self.description, self.keywords).to_lowercase()
    }
}

#[derive(Clone, Debug)]
pub struct ResultItem {
    pub candidate: Candidate,
    pub score: i32,
}

pub fn search(
    query: &str,
    candidates: &[Candidate],
    current: Platform,
    limit: usize,
    all_platforms: bool,
) -> Vec<ResultItem> {
    let query = normalize(query);
    let tokens = tokens(&query);
    if tokens.is_empty() || limit == 0 {
        return Vec::new();
    }
    let expansions = expand(&tokens, &query);

    let mut matches: Vec<ResultItem> = candidates
        .iter()
        .filter(|candidate| all_platforms || candidate.platforms.matches(current))
        .filter_map(|candidate| {
            let score = score(candidate, &query, &tokens, &expansions);
            (score > 0).then(|| ResultItem {
                candidate: candidate.clone(),
                score,
            })
        })
        .collect();

    matches.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| {
                source_rank(&left.candidate.source).cmp(&source_rank(&right.candidate.source))
            })
            .then_with(|| left.candidate.command.cmp(&right.candidate.command))
    });
    matches.truncate(limit);
    matches
}

fn source_rank(source: &Source) -> u8 {
    match source {
        Source::Builtin => 0,
        Source::Config(_) => 1,
        Source::Alias(_) => 2,
        Source::Path(_) => 3,
    }
}

fn score(
    candidate: &Candidate,
    query: &str,
    query_tokens: &[String],
    expansions: &[String],
) -> i32 {
    let haystack = candidate.searchable_text();
    let haystack_tokens = tokens(&haystack);
    let command_tokens = tokens(&candidate.command);
    let mut total = 0;

    if haystack.contains(query) {
        total += 20;
    }
    for token in query_tokens {
        if haystack_tokens.iter().any(|word| word == token) {
            total += 9;
        } else if haystack_tokens
            .iter()
            .any(|word| word.starts_with(token) || token.starts_with(word))
        {
            total += 3;
        }
    }
    for word in expansions {
        if haystack_tokens.iter().any(|token| token == word) {
            total += 6;
        }
        // A semantic match in the command name itself (for example the
        // `minetoggle` part of a mining request) is more useful than an
        // incidental mention in a long description.
        if command_tokens.iter().any(|token| token == word) {
            total += 5;
        }
    }
    if normalize(&candidate.command).starts_with(query) {
        total += 15;
    }
    // A result needs an actual shared term or at least two useful semantic expansions.
    let direct_matches = query_tokens
        .iter()
        .filter(|token| !is_generic_context_token(token))
        .filter(|token| haystack_tokens.iter().any(|word| word == *token))
        .count();
    let expansion_matches = expansions
        .iter()
        .filter(|word| haystack_tokens.iter().any(|token| token == *word))
        .count();
    if direct_matches == 0 && expansion_matches < 2 {
        return 0;
    }
    total
}

fn is_generic_context_token(token: &str) -> bool {
    matches!(
        token,
        "mac"
            | "macos"
            | "linux"
            | "computer"
            | "machine"
            | "pc"
            | "system"
            | "command"
            | "terminal"
            | "shell"
    )
}

fn expand(tokens: &[String], query: &str) -> Vec<String> {
    let mut result = Vec::new();
    let words: Vec<&str> = query.split_whitespace().collect();
    let asks_to_turn = words.contains(&"turn");
    let asks_to_turn_off = asks_to_turn && words.contains(&"off");
    let asks_to_turn_on = asks_to_turn && words.contains(&"on");
    if query.contains("turn off")
        || asks_to_turn_off
        || query.contains("power off")
        || query.contains("shut down")
    {
        result.extend(
            [
                "shutdown", "poweroff", "halt", "power", "down", "toggle", "control",
            ]
            .into_iter()
            .map(String::from),
        );
    }
    if query.contains("turn on") || asks_to_turn_on {
        result.extend(
            ["start", "enable", "resume", "on", "toggle", "control"]
                .into_iter()
                .map(String::from),
        );
    }
    for token in tokens {
        let related: &[&str] = match token.as_str() {
            "computer" | "machine" | "pc" => &["system", "laptop"],
            "sleep" => &["suspend", "hibernate"],
            "restart" => &["reboot"],
            "delete" => &["remove", "rm"],
            "copy" => &["duplicate", "cp"],
            "move" | "rename" => &["mv"],
            "search" => &["find", "grep", "locate"],
            "find" => &["search", "locate", "grep"],
            "archive" | "compress" => &["tar", "gzip", "zip"],
            "extract" | "unpack" => &["tar", "unzip"],
            "process" => &["program", "task", "pid"],
            "internet" | "network" => &["connection", "host", "ip"],
            "port" | "ports" => &["listen", "socket"],
            "permission" => &["chmod", "executable"],
            "mining" | "miner" => &["mine", "minetoggle", "kryptex", "tari"],
            _ => &[],
        };
        result.extend(related.iter().map(|word| (*word).to_string()));
    }
    result.sort();
    result.dedup();
    result
}

pub fn normalize(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn tokens(value: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "a",
        "an",
        "the",
        "my",
        "me",
        "to",
        "for",
        "of",
        "on",
        "in",
        "with",
        "please",
        "how",
        "do",
        "turn",
        "i",
        "want",
        "can",
        "you",
        "show",
        "list",
        "see",
        "what",
        "is",
        "using",
        "all",
        "current",
        "currently",
    ];
    normalize(value)
        .split_whitespace()
        .filter(|word| !STOP_WORDS.contains(word))
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog;

    fn commands(query: &str, platform: Platform) -> Vec<String> {
        search(query, &catalog::builtins(), platform, 5, false)
            .into_iter()
            .map(|item| item.candidate.command)
            .collect()
    }

    #[test]
    fn normalizes_human_queries() {
        assert_eq!(normalize("Turn OFF my computer!"), "turn off my computer");
        assert_eq!(tokens("please find the files"), vec!["find", "files"]);
    }

    #[test]
    fn ranks_shutdown_queries_above_unrelated_results() {
        let results = commands("turn off my computer", Platform::Linux);
        assert_eq!(results[0], "sudo poweroff");
        assert!(results
            .iter()
            .any(|command| command == "sudo shutdown -h now"));
    }

    #[test]
    fn supports_macos_specific_results() {
        let results = commands("put my mac to sleep", Platform::Mac);
        assert_eq!(results[0], "pmset sleepnow");
        assert_eq!(results, vec!["pmset sleepnow"]);
    }

    #[test]
    fn limits_and_filters_platforms() {
        let results = search(
            "network ip address",
            &catalog::builtins(),
            Platform::Linux,
            2,
            false,
        );
        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .all(|item| item.candidate.platforms != Platform::Mac));
    }

    #[test]
    fn can_include_other_platforms_explicitly() {
        let results = search(
            "put my mac to sleep",
            &catalog::builtins(),
            Platform::Linux,
            5,
            true,
        );
        assert!(results
            .iter()
            .any(|item| item.candidate.command == "pmset sleepnow"));
    }

    #[test]
    fn broad_regression_queries_find_expected_commands() {
        let cases = [
            ("find text in files", Platform::Linux, "rg 'pattern'"),
            (
                "find a file by name",
                Platform::Linux,
                "find . -name 'pattern'",
            ),
            ("what is using disk space", Platform::Linux, "du -sh *"),
            ("show remaining disk space", Platform::Linux, "df -h"),
            (
                "make a folder",
                Platform::Linux,
                "mkdir -p path/to/directory",
            ),
            ("rename a file", Platform::Linux, "mv old-name new-name"),
            ("safely delete file", Platform::Linux, "rm -i file"),
            (
                "compress a directory",
                Platform::Linux,
                "tar -czf archive.tar.gz directory",
            ),
            ("extract zip archive", Platform::Linux, "unzip archive.zip"),
            ("show running processes", Platform::Linux, "ps aux"),
            ("stop a process", Platform::Linux, "kill PID"),
            ("monitor cpu memory", Platform::Linux, "top"),
            ("show my ip address", Platform::Linux, "ip addr"),
            ("show listening ports", Platform::Linux, "ss -tulpn"),
            ("test internet connection", Platform::Linux, "ping host"),
            ("make a web request", Platform::Linux, "curl -I URL"),
            ("connect to remote server", Platform::Linux, "ssh user@host"),
            (
                "make shell script executable",
                Platform::Linux,
                "chmod +x script",
            ),
            ("see git changes", Platform::Linux, "git status"),
            (
                "show git history",
                Platform::Linux,
                "git log --oneline --decorate -n 20",
            ),
            ("run rust tests", Platform::Linux, "cargo test"),
            (
                "create python venv",
                Platform::Linux,
                "python3 -m venv .venv",
            ),
            ("lock the screen", Platform::Linux, "loginctl lock-session"),
            ("restart computer", Platform::Linux, "sudo reboot"),
            (
                "put computer to sleep",
                Platform::Linux,
                "systemctl suspend",
            ),
            ("mac network interface", Platform::Mac, "ifconfig"),
            (
                "mac listening ports",
                Platform::Mac,
                "lsof -nP -iTCP -sTCP:LISTEN",
            ),
        ];
        for (query, platform, expected) in cases {
            let found = commands(query, platform);
            assert!(
                found.iter().any(|command| command == expected),
                "{query:?} did not return {expected:?}; got {found:?}"
            );
        }
    }
}

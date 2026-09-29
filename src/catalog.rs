use crate::engine::{Candidate, Platform, Source};

pub fn builtins() -> Vec<Candidate> {
    let mut items = Vec::new();
    let mut add = |command: &str, description: &str, keywords: &str, platforms: Platform| {
        items.push(Candidate::builtin(
            command,
            description,
            keywords,
            platforms,
        ));
    };

    // Power and sessions. Results are suggestions only; clifind never runs them.
    add(
        "sudo poweroff",
        "Shut down the computer immediately.",
        "turn off computer shutdown power down halt",
        Platform::Linux,
    );
    add(
        "sudo shutdown -h now",
        "Shut down the computer now.",
        "turn off computer shutdown power down halt",
        Platform::Both,
    );
    add(
        "systemctl poweroff",
        "Ask systemd to power off the machine.",
        "turn off computer shutdown power down halt",
        Platform::Linux,
    );
    add(
        "sudo reboot",
        "Restart the computer.",
        "restart reboot computer system",
        Platform::Both,
    );
    add(
        "systemctl suspend",
        "Suspend the Linux system to memory.",
        "sleep suspend computer laptop",
        Platform::Linux,
    );
    add(
        "pmset sleepnow",
        "Put the Mac to sleep immediately.",
        "sleep suspend computer mac laptop",
        Platform::Mac,
    );
    add(
        "loginctl lock-session",
        "Lock the current Linux desktop session.",
        "lock screen session",
        Platform::Linux,
    );
    add("/System/Library/CoreServices/Menu\\ Extras/User.menu/Contents/Resources/CGSession -suspend", "Open the macOS lock screen.", "lock screen session mac", Platform::Mac);

    // Files, text, and archives.
    add(
        "rg 'pattern'",
        "Recursively search file contents with ripgrep.",
        "find search text files grep contents",
        Platform::Both,
    );
    add(
        "find . -name 'pattern'",
        "Find files and directories by name.",
        "find search locate files folders name",
        Platform::Both,
    );
    add(
        "fd pattern",
        "Quickly find files by name (requires fd).",
        "find search locate files folders name",
        Platform::Both,
    );
    add(
        "ls -lah",
        "List files, including hidden files, with readable sizes.",
        "list show files folders directory hidden",
        Platform::Both,
    );
    add(
        "du -sh *",
        "Show the size of each item in the current directory.",
        "disk usage space large files folders",
        Platform::Both,
    );
    add(
        "df -h",
        "Show free and used space on mounted filesystems.",
        "disk free space storage usage",
        Platform::Both,
    );
    add(
        "mkdir -p path/to/directory",
        "Create a directory and any missing parent directories.",
        "make create new directory folder",
        Platform::Both,
    );
    add(
        "cp -a source destination",
        "Copy files while preserving attributes where supported.",
        "copy files directory folder",
        Platform::Linux,
    );
    add(
        "cp -R source destination",
        "Copy a directory and its contents.",
        "copy files directory folder",
        Platform::Mac,
    );
    add(
        "mv old-name new-name",
        "Move or rename a file or directory.",
        "move rename files folder",
        Platform::Both,
    );
    add(
        "rm -i file",
        "Interactively remove a file after confirmation.",
        "delete remove file safe",
        Platform::Both,
    );
    add(
        "tar -czf archive.tar.gz directory",
        "Create a gzip-compressed tar archive.",
        "archive compress zip backup folder",
        Platform::Both,
    );
    add(
        "tar -xzf archive.tar.gz",
        "Extract a gzip-compressed tar archive.",
        "extract unpack archive tar gzip",
        Platform::Both,
    );
    add(
        "unzip archive.zip",
        "Extract a ZIP archive.",
        "extract unpack zip archive",
        Platform::Both,
    );

    // Processes and networking.
    add(
        "ps aux",
        "Show running processes.",
        "show list running processes programs tasks",
        Platform::Both,
    );
    add(
        "pgrep -af name",
        "Find running processes by name and show their command lines.",
        "find search process program running",
        Platform::Both,
    );
    add(
        "kill PID",
        "Request that a process stops gracefully.",
        "stop kill process program task",
        Platform::Both,
    );
    add(
        "pkill -f pattern",
        "Stop processes whose command line matches a pattern.",
        "stop kill process program task",
        Platform::Both,
    );
    add(
        "top",
        "Interactively inspect CPU and memory use by process.",
        "cpu memory process monitor performance",
        Platform::Both,
    );
    add(
        "htop",
        "Interactively inspect processes (requires htop).",
        "cpu memory process monitor performance",
        Platform::Both,
    );
    add(
        "ip addr",
        "Show Linux network interfaces and addresses.",
        "ip address network wifi ethernet interface",
        Platform::Linux,
    );
    add(
        "ifconfig",
        "Show network interface configuration.",
        "ip address network wifi ethernet interface mac",
        Platform::Mac,
    );
    add(
        "ss -tulpn",
        "List listening Linux network sockets.",
        "ports listening network server connection",
        Platform::Linux,
    );
    add(
        "lsof -nP -iTCP -sTCP:LISTEN",
        "List listening TCP ports on macOS.",
        "ports listening network server connection mac",
        Platform::Mac,
    );
    add(
        "ping host",
        "Test whether a host is reachable over the network.",
        "network internet connection test host",
        Platform::Both,
    );
    add(
        "curl -I URL",
        "Fetch HTTP response headers from a URL.",
        "http website url request headers network",
        Platform::Both,
    );
    add(
        "ssh user@host",
        "Open a secure remote shell.",
        "remote login server connect ssh",
        Platform::Both,
    );

    // Development and permissions.
    add(
        "chmod +x script",
        "Make a script executable.",
        "permission executable run script chmod",
        Platform::Both,
    );
    add(
        "git status",
        "Show changed, staged, and untracked files in a Git repository.",
        "git changes status repository",
        Platform::Both,
    );
    add(
        "git log --oneline --decorate -n 20",
        "Show recent Git commits compactly.",
        "git history commits log",
        Platform::Both,
    );
    add(
        "git switch branch",
        "Switch to an existing Git branch.",
        "git change switch branch checkout",
        Platform::Both,
    );
    add(
        "git pull --ff-only",
        "Update the current Git branch without creating a merge commit.",
        "git update pull sync repository",
        Platform::Both,
    );
    add(
        "git grep 'pattern'",
        "Search tracked Git files for text.",
        "git search find text files",
        Platform::Both,
    );
    add(
        "cargo test",
        "Compile and run Rust tests.",
        "rust test cargo verify",
        Platform::Both,
    );
    add(
        "python3 -m venv .venv",
        "Create a Python virtual environment.",
        "python virtual environment venv setup",
        Platform::Both,
    );

    items
}

#[allow(dead_code)]
fn _source_is_used(_: Source) {}

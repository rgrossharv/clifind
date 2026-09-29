# clifind

`clifind` is a tiny, fast command search engine for macOS and Linux. Describe what you want in ordinary language and it returns a few terminal commands with short descriptions. It searches a built-in cross-platform catalogue first, then safely indexes commands already available on the machine.

```console
$ clifind turn off my computer
1. sudo poweroff — Shut down the computer immediately. [linux; built-in]
2. sudo shutdown -h now — Shut down the computer now. [macos/linux; built-in]
3. systemctl poweroff — Ask systemd to power off the machine. [linux; built-in]

Suggestions only — clifind never executes a result.
```

## Design

- **Fast and local.** Normal searches use a small in-memory catalogue, token matching, phrase matching, and a lightweight synonym map. There is no network request, database, telemetry, shell evaluation, or model load.
- **Platform-aware.** It shows commands for the current operating system by default. Pass `--all-platforms` to compare macOS and Linux options.
- **Knows your own tools.** `clifind` reads executable commands in `PATH`, `~/.local/bin`, and `~/bin`; simple aliases from common Bash/Zsh/Fish config files; and an optional explicit config file. Discovery only reads files—it never runs a discovered command.
- **Careful by default.** It is a recommender, not a command runner. Potentially destructive commands are displayed, never executed.
- **Optional local explanation.** `--explain` is opt-in and uses no model unless requested. It accepts only a local GGUF model whose tensor count is measured at no more than **1,000,000,000 parameters**, and stops its local runner after three seconds. The normal search engine remains fully mechanistic.

## Install

After a release has been published, install the matching verified release binary with:

```sh
curl -fsSL https://raw.githubusercontent.com/rgrossharv/clifind/main/install.sh | sh
```

The installer supports Linux x86_64 and both Apple Silicon and Intel Macs. On another supported Rust host, install from source:

```sh
cargo install --git https://github.com/rgrossharv/clifind.git clifind
```

To install a specific released version, set `CLIFIND_VERSION`, e.g. `CLIFIND_VERSION=0.1.0 sh install.sh`. The installer verifies `SHA256SUMS` before placing the executable in `${CLIFIND_INSTALL_DIR:-$HOME/.local/bin}`.

## Use

```sh
clifind find text in files
clifind show listening ports
clifind make a compressed backup
clifind --all-platforms put my mac to sleep
clifind --json see git changes
clifind -n 6 turn mining on
```

`--no-custom` disables every machine-specific source, which is useful for repeatable scripts and tests.

### Custom commands

An executable called `minetoggle` in `~/.local/bin` is found automatically, even in a non-login shell. For scripts, initial comment lines become a searchable description. For example, the comment `# Remote-friendly on/off control for the configured miner.` makes `clifind turn mining on` find `minetoggle`.

For commands without useful script comments—or aliases/functions that do more than simple aliases—add a tab-separated file at `~/.config/clifind/commands.tsv` (or set `CLIFIND_COMMANDS_FILE`):

```tsv
# command<TAB>description<TAB>optional searchable keywords
minetoggle	Start, stop, or check the configured miner.	mining miner tari kryptex on off status
deploy-staging	Deploy the current app to staging.	deploy release staging app
```

The file is deliberately explicit and portable, so custom commands work on any machine on which it is installed.

### Optional small local model

The deterministic results are the source of truth; a model may only summarize the top results.

```sh
export CLIFIND_LLM_MODEL="$HOME/models/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf"
export CLIFIND_LLM_BIN="$HOME/bin/llama-cli"
clifind --explain turn off my computer
```

`clifind` parses the GGUF header and tensor descriptions before starting the runner. It rejects model files over 1B tensor elements, does not send prompts off-device, and has a hard three-second runner timeout. A wrong, missing, oversized, or unsupported model leaves the normal results intact and reports a concise warning on stderr.

## Development

This project intentionally has no crate dependencies, keeping startup and audit surface small.

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
./target/release/clifind turn off my computer
```

The regression suite exercises power/session, filesystem, archive, process, network, permissions, Git, Rust, Python, macOS-only, Linux-only, JSON, custom `minetoggle`-style commands, alias parsing, and model-cap parsing. Run `cargo test` before release.

## Publishing

`publish.sh` is intentionally ignored and must stay local: it checks formatting, tests, Clippy, builds a release binary, prints staged and remote state, commits all non-ignored work, pushes the branch, and can optionally create a version tag. Because it is ignored, it is never added to or displayed by GitHub.

```sh
./publish.sh "Initial clifind release"
PUBLISH_TAG=v0.1.0 ./publish.sh "Release 0.1.0"
```

Pushing a `v*` tag starts the GitHub Actions release workflow. It builds the downloadable Linux x86_64 and macOS (Intel/Apple Silicon) archives plus `SHA256SUMS`; those archives are what the curl installer downloads.

## License

[MIT](LICENSE)

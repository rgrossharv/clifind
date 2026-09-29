# clifind

`clifind` finds terminal commands from plain-language descriptions. It searches a built-in catalogue and commands available on your machine. Results are suggestions; clifind never runs them.

```sh
clifind find text in files
clifind show listening ports
clifind --all-platforms put my mac to sleep
```

## Install

On Linux x86_64 or Apple Silicon macOS, install the latest release with:

```sh
curl -fsSL https://raw.githubusercontent.com/rgrossharv/clifind/main/install.sh | sh
```

The binary is installed to `~/.local/bin`. Add that directory to your `PATH` if needed. To choose a version, for example:

```sh
curl -fsSL https://raw.githubusercontent.com/rgrossharv/clifind/main/install.sh | CLIFIND_VERSION=0.1.0 sh
```

You can also install from source on a Rust-supported platform:

```sh
cargo install --git https://github.com/rgrossharv/clifind.git clifind
```

## Custom commands

Executables in `PATH`, `~/.local/bin`, and `~/bin` are discovered automatically. To add descriptions or keywords, create `~/.config/clifind/commands.tsv`:

```tsv
# command<TAB>description<TAB>optional keywords
deploy-staging	Deploy the app to staging.	deploy release staging
```

Set `CLIFIND_COMMANDS_FILE` to use a different file. Use `--no-custom` to search only the built-in catalogue.

## Build

```sh
cargo build --release
```

## License

[MIT](LICENSE)

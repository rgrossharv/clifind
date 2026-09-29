#!/bin/sh
# Install a verified clifind release. Intended for:
#   curl -fsSL https://raw.githubusercontent.com/rgrossharv/clifind/main/install.sh | sh
set -eu

repository="rgrossharv/clifind"
install_dir="${CLIFIND_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "clifind install: $*"; }
die() { say "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "requires $1"; }

need curl
need tar

os="$(uname -s)"
arch="$(uname -m)"
case "$os/$arch" in
  Linux/x86_64|Linux/amd64) target="x86_64-unknown-linux-musl" ;;
  Darwin/arm64|Darwin/aarch64) target="aarch64-apple-darwin" ;;
  *) die "no prebuilt release for $os/$arch; install from source with: cargo install --git https://github.com/$repository.git clifind" ;;
esac

if [ -n "${CLIFIND_VERSION:-}" ]; then
  version="${CLIFIND_VERSION#v}"
else
  version="$(curl -fsSL "https://api.github.com/repos/$repository/releases/latest" | sed -n 's/^[[:space:]]*"tag_name"[[:space:]]*:[[:space:]]*"v\{0,1\}\([^"]*\)".*/\1/p' | head -n 1)"
fi
[ -n "$version" ] || die "could not determine the latest release; set CLIFIND_VERSION explicitly"

archive="clifind-${version}-${target}.tar.gz"
base="https://github.com/$repository/releases/download/v$version"
temporary="$(mktemp -d "${TMPDIR:-/tmp}/clifind.XXXXXX")"
cleanup() { rm -rf "$temporary"; }
trap cleanup EXIT HUP INT TERM

say "downloading clifind v$version for $target"
curl -fL --retry 3 -o "$temporary/$archive" "$base/$archive"
curl -fL --retry 3 -o "$temporary/SHA256SUMS" "$base/SHA256SUMS"

expected="$(awk -v file="$archive" '$2 == file || $2 == "*" file { print $1; exit }' "$temporary/SHA256SUMS")"
[ -n "$expected" ] || die "checksum for $archive is missing from SHA256SUMS"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$temporary/$archive" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  actual="$(shasum -a 256 "$temporary/$archive" | awk '{print $1}')"
else
  die "requires sha256sum or shasum to verify the download"
fi
[ "$actual" = "$expected" ] || die "checksum verification failed"

tar -xzf "$temporary/$archive" -C "$temporary"
[ -x "$temporary/clifind" ] || die "release archive did not contain an executable clifind"
mkdir -p "$install_dir"
install -m 755 "$temporary/clifind" "$install_dir/clifind"
say "installed $install_dir/clifind"
case ":${PATH}:" in
  *":$install_dir:"*) ;;
  *) say "add $install_dir to PATH, then open a new shell" ;;
esac

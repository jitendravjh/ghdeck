#!/bin/sh
# Installs ghdeck for this machine from the latest GitHub release.
#   curl -fsSL https://raw.githubusercontent.com/jitendravjh/ghdeck/main/install.sh | sh
# GHDECK_VERSION picks a release like v0.2.0, GHDECK_BIN_DIR picks where the binary goes.
set -eu

repo="jitendravjh/ghdeck"
version="${GHDECK_VERSION:-latest}"
bin_dir="${GHDECK_BIN_DIR:-/usr/local/bin}"

say() { printf '%s\n' "$*"; }
die() {
  printf 'ghdeck install: %s\n' "$*" >&2
  exit 1
}
need() { command -v "$1" >/dev/null 2>&1 || die "needs $1"; }

case "$(uname -s)" in
  Darwin) os="apple-darwin" ;;
  Linux) os="unknown-linux-musl" ;;
  *) die "there is no build for $(uname -s), only macOS and Linux" ;;
esac
case "$(uname -m)" in
  arm64 | aarch64) arch="aarch64" ;;
  x86_64 | amd64) arch="x86_64" ;;
  *) die "there is no build for $(uname -m) cpus" ;;
esac
# a shell running under rosetta reports x86_64 on apple silicon, the native build is better
if [ "$os" = "apple-darwin" ] && [ "$arch" = "x86_64" ] && [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || true)" = "1" ]; then
  arch="aarch64"
fi
file="ghdeck-$arch-$os.tar.gz"

case "$version" in
  latest) base="https://github.com/$repo/releases/latest/download" ;;
  v*) base="https://github.com/$repo/releases/download/$version" ;;
  *) base="https://github.com/$repo/releases/download/v$version" ;;
esac

need curl
need tar
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

say "downloading $file"
curl -fsSL "$base/$file" -o "$tmp/$file" || die "could not download $base/$file"
curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS" || die "could not download the checksums"

want="$(awk -v f="$file" '$2 == f { print $1 }' "$tmp/SHA256SUMS")"
[ -n "$want" ] || die "$file is not listed in SHA256SUMS"
if command -v sha256sum >/dev/null 2>&1; then
  got="$(sha256sum "$tmp/$file" | cut -d ' ' -f 1)"
elif command -v shasum >/dev/null 2>&1; then
  got="$(shasum -a 256 "$tmp/$file" | cut -d ' ' -f 1)"
else
  die "needs sha256sum or shasum to check the download"
fi
[ "$got" = "$want" ] || die "checksum mismatch for $file, not installing it"

tar -xzf "$tmp/$file" -C "$tmp"
[ -f "$tmp/ghdeck" ] || die "the archive had no ghdeck binary in it"

sudo=""
if ! mkdir -p "$bin_dir" 2>/dev/null || [ ! -w "$bin_dir" ]; then
  need sudo
  say "$bin_dir needs admin rights, so sudo will ask for your password"
  sudo="sudo"
fi
$sudo mkdir -p "$bin_dir"
$sudo install -m 755 "$tmp/ghdeck" "$bin_dir/ghdeck"

say "installed $("$bin_dir/ghdeck" --version) to $bin_dir/ghdeck"
case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *) say "$bin_dir is not on your PATH yet, add it there to run ghdeck by name" ;;
esac
first="$(command -v ghdeck 2>/dev/null || true)"
if [ -n "$first" ] && [ "$first" != "$bin_dir/ghdeck" ]; then
  say "note: $first comes first on your PATH, so that copy runs when you type ghdeck"
fi
say "next, connect your account: https://ghdeck.jitendravjh.in/docs/#connect-your-account"

#!/bin/sh
# Trackly installer — downloads the latest prebuilt binary for your platform.
#
#   curl -fsSL https://raw.githubusercontent.com/vehutech/trackly/main/scripts/install.sh | sh
#
# Env overrides:
#   TRACKLY_VERSION  pin a version (e.g. v0.1.0); defaults to the latest release
#   TRACKLY_BIN_DIR  install location; defaults to ~/.local/bin
set -eu

REPO="vehutech/trackly"
BIN_DIR="${TRACKLY_BIN_DIR:-$HOME/.local/bin}"

err() { printf 'error: %s\n' "$1" >&2; exit 1; }

# --- detect platform ---
os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux)  os_part="unknown-linux-gnu" ;;
  Darwin) os_part="apple-darwin" ;;
  *) err "unsupported OS '$os'. On Windows, download the .zip from https://github.com/$REPO/releases" ;;
esac

case "$arch" in
  x86_64|amd64) arch_part="x86_64" ;;
  arm64|aarch64) arch_part="aarch64" ;;
  *) err "unsupported architecture '$arch'" ;;
esac

# Linux prebuilds are x86_64 only for now.
if [ "$os" = "Linux" ] && [ "$arch_part" != "x86_64" ]; then
  err "no prebuilt Linux binary for '$arch' yet — build from source: https://github.com/$REPO#install"
fi

target="${arch_part}-${os_part}"

# --- resolve version ---
version="${TRACKLY_VERSION:-}"
if [ -z "$version" ]; then
  version="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
  [ -n "$version" ] || err "could not determine the latest version — set TRACKLY_VERSION"
fi

asset="trackly-${version}-${target}.tar.gz"
url="https://github.com/$REPO/releases/download/${version}/${asset}"

printf 'Installing trackly %s (%s)\n' "$version" "$target"

# --- download & extract ---
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fsSL "$url" -o "$tmp/$asset" || err "download failed: $url"
tar -xzf "$tmp/$asset" -C "$tmp"

mkdir -p "$BIN_DIR"
install -m 0755 "$tmp/trackly" "$BIN_DIR/trackly"
# The MCP server ships in the same archive (for native agent integration).
[ -f "$tmp/trackly-mcp" ] && install -m 0755 "$tmp/trackly-mcp" "$BIN_DIR/trackly-mcp"

printf '\n✓ installed trackly (and trackly-mcp) to %s\n' "$BIN_DIR"
case ":$PATH:" in
  *":$BIN_DIR:"*) : ;;
  *) printf '\n! %s is not on your PATH. Add this to your shell profile:\n    export PATH="%s:$PATH"\n' "$BIN_DIR" "$BIN_DIR" ;;
esac
printf '\nRun: trackly --help\n'

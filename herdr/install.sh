#!/usr/bin/env bash
# herdr `[[build]]` step: download the prebuilt herdr-sheep binary for this platform from the
# matching GitHub Release into the plugin's bin/ dir, so installing the plugin needs no Rust
# toolchain. Runs on `herdr plugin install`; `herdr plugin link` skips build commands, so a
# local checkout stages its own binary with `just install`.
#
# The build runs with the plugin checkout as the working directory and build commands may not
# receive the runtime env, so the plugin root is resolved from this script's own location.
set -euo pipefail

NAME="herdr-sheep"
REPO="huketo/herdr-sheep"

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="$ROOT/bin"

# The release tag matches the manifest version, so a checkout always pulls its own release.
# Same extraction as herdr/check-version.sh, which is what keeps the two in step: the first
# `version = "..."` before the next table header.
VERSION="$(sed -nE '1,/^\[[a-z]/ s/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$ROOT/herdr-plugin.toml" | head -1)"
if [[ -z "$VERSION" ]]; then
  echo "$NAME: cannot read version from $ROOT/herdr-plugin.toml" >&2
  exit 1
fi
TAG="v${VERSION}"

# Map the running platform to a release target triple.
os="$(uname -s)"
arch="$(uname -m)"
case "$os-$arch" in
  Darwin-arm64) target="aarch64-apple-darwin" ;;
  Darwin-x86_64) target="x86_64-apple-darwin" ;;
  Linux-aarch64 | Linux-arm64) target="aarch64-unknown-linux-musl" ;;
  Linux-x86_64) target="x86_64-unknown-linux-musl" ;;
  *)
    echo "$NAME: no prebuilt binary for $os-$arch — build from source with 'cargo build --release'" >&2
    exit 1
    ;;
esac

archive="${NAME}-${target}.tar.gz"
# upload-rust-binary-action's checksum sidecar drops the archive extension.
checksum="${NAME}-${target}.sha256"
base="https://github.com/${REPO}/releases/download/${TAG}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Release assets are eventually consistent: GitHub's CDN can 404 for a few minutes after a
# release publishes. Retry on every error, 404 included, so installing right after a release
# does not fail spuriously.
dl() { curl -fsSL --retry 5 --retry-delay 3 --retry-all-errors --retry-connrefused "$1" -o "$2"; }

echo "$NAME: downloading $archive ($TAG)"
dl "$base/$archive" "$tmp/$archive"
dl "$base/$checksum" "$tmp/$checksum"

echo "$NAME: verifying checksum"
expected="$(awk '{print $1}' "$tmp/$checksum")"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$tmp/$archive" | awk '{print $1}')"
else
  actual="$(shasum -a 256 "$tmp/$archive" | awk '{print $1}')"
fi
if [[ "$expected" != "$actual" ]]; then
  echo "$NAME: checksum mismatch (expected $expected, got $actual)" >&2
  exit 1
fi

mkdir -p "$BIN_DIR"
tar -xzf "$tmp/$archive" -C "$tmp"
install -m 0755 "$tmp/$NAME" "$BIN_DIR/$NAME"
echo "$NAME: installed $BIN_DIR/$NAME"

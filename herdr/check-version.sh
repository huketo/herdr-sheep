#!/usr/bin/env bash
# The release tag, Cargo.toml, and herdr-plugin.toml all have to say the same version:
# herdr/install.sh builds the release download URL from the manifest version, so a manifest
# that disagrees with its tag installs a 404.
#
# Usage: herdr/check-version.sh [expected]   (`expected` accepts a leading `v`)
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

# First `version = "..."` before the next table header, so a dependency's version cannot win.
read_version() {
  sed -nE '1,/^\[[a-z]/ s/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$1" | head -1
}

cargo_version="$(read_version "$ROOT/Cargo.toml")"
manifest_version="$(read_version "$ROOT/herdr-plugin.toml")"

if [[ -z "$cargo_version" || -z "$manifest_version" ]]; then
  echo "version check: cannot read a version (Cargo.toml: '$cargo_version', herdr-plugin.toml: '$manifest_version')" >&2
  exit 1
fi

if [[ "$cargo_version" != "$manifest_version" ]]; then
  echo "version check: Cargo.toml says $cargo_version, herdr-plugin.toml says $manifest_version" >&2
  exit 1
fi

expected="${1:-}"
expected="${expected#v}"
if [[ -n "$expected" && "$expected" != "$cargo_version" ]]; then
  echo "version check: tag says $expected, the crate and manifest say $cargo_version" >&2
  exit 1
fi

echo "version check: $cargo_version"

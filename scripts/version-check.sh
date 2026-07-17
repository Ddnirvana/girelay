#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
girelay_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/crates/girelay/Cargo.toml" | head -1)"
test -n "$girelay_version"
grep -E "^## (\\[$girelay_version\\]|$girelay_version)( |$)" "$ROOT/CHANGELOG.md" >/dev/null
echo "Version $girelay_version is consistent with CHANGELOG.md."

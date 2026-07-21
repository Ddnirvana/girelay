#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
girelay_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/crates/girelay/Cargo.toml" | head -1)"
test -n "$girelay_version"
grep -E "^## (\\[$girelay_version\\]|$girelay_version)( |$)" "$ROOT/CHANGELOG.md" >/dev/null

if [ -n "${GIRELAY_RELEASE_TAG:-}" ]; then
  expected_tag="v$girelay_version"
  if [ "$GIRELAY_RELEASE_TAG" != "$expected_tag" ]; then
    echo "Release tag $GIRELAY_RELEASE_TAG does not match Cargo version $girelay_version (expected $expected_tag)." >&2
    exit 1
  fi
fi

echo "Version $girelay_version is consistent with CHANGELOG.md."

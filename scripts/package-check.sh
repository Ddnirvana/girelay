#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
COPY="$TMP/girelay"
mkdir -p "$COPY"

git -C "$ROOT" ls-files --cached --others --exclude-standard -z \
  | rsync -a --from0 --files-from=- "$ROOT/" "$COPY/"

git -C "$COPY" init -b package-check >/dev/null
git -C "$COPY" config user.email package-check@example.invalid
git -C "$COPY" config user.name "girelay package check"
git -C "$COPY" add .
git -C "$COPY" commit -m "package check snapshot" >/dev/null

cargo package --manifest-path "$COPY/crates/girelay/Cargo.toml" --allow-dirty --offline

echo "Package checks passed in a committed temporary copy."

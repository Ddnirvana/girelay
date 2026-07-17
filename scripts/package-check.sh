#!/usr/bin/env bash
set -euo pipefail

# macOS BSD tar rejects the Linux-common C.UTF-8 locale.
export LANG=C
export LC_ALL=C
export LC_CTYPE=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
COPY="$TMP/girelay"
mkdir -p "$COPY"

git -C "$ROOT" ls-files --cached --others --exclude-standard -z \
  | while IFS= read -r -d '' path; do
      if [ -e "$ROOT/$path" ] || [ -L "$ROOT/$path" ]; then
        printf '%s\0' "$path"
      fi
    done \
  | tar --null -T - -cf - -C "$ROOT" 2>/dev/null \
  | tar -xf - -C "$COPY"

git -C "$COPY" init -b package-check >/dev/null
git -C "$COPY" config user.email package-check@example.invalid
git -C "$COPY" config user.name "girelay package check"
git -C "$COPY" add .
git -C "$COPY" commit -m "package check snapshot" >/dev/null

cargo package --manifest-path "$COPY/crates/girelay/Cargo.toml" --allow-dirty --offline

echo "Package checks passed in a committed temporary copy."

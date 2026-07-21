#!/usr/bin/env bash
set -euo pipefail

# macOS BSD tar does not recognize the Linux-common C.UTF-8 locale.
export LANG=C
export LC_ALL=C
export LC_CTYPE=C

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TARGET="${1:-$(rustc -vV | awk '/host:/ {print $2}')}"
BIN="girelay"
ARCHIVE="girelay-$TARGET.tar.gz"
DIST="$ROOT/dist"

mkdir -p "$DIST"

cargo build --release --target "$TARGET"
cp "target/$TARGET/release/$BIN" "$DIST/$BIN"
cp README.md LICENSE "$DIST/"

(
  cd "$DIST"
  tar -czf "$ARCHIVE" "$BIN" README.md LICENSE
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$ARCHIVE" > "$ARCHIVE.sha256"
  else
    shasum -a 256 "$ARCHIVE" > "$ARCHIVE.sha256"
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c "$ARCHIVE.sha256"
  else
    shasum -a 256 -c "$ARCHIVE.sha256"
  fi
)

echo "Wrote $DIST/$ARCHIVE"
echo "Wrote $DIST/$ARCHIVE.sha256"

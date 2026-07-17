#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TARGET="${1:-$(rustc -vV | awk '/host:/ {print $2}')}"
BIN="girelay"
ARCHIVE="girelay-$TARGET.tar.gz"
DIST="$ROOT/dist"

mkdir -p "$DIST"

cargo build --release --target "$TARGET"
cp "target/$TARGET/release/$BIN" "$DIST/$BIN"

(
  cd "$DIST"
  tar -czf "$ARCHIVE" "$BIN"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$ARCHIVE" > "$ARCHIVE.sha256"
  else
    shasum -a 256 "$ARCHIVE" > "$ARCHIVE.sha256"
  fi
)

echo "Wrote $DIST/$ARCHIVE"
echo "Wrote $DIST/$ARCHIVE.sha256"

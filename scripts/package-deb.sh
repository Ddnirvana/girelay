#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="${1:-build}"
TARGET="${2:-x86_64-unknown-linux-gnu}"
ARCH="${3:-amd64}"
VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/crates/girelay/Cargo.toml" | head -1)"
MAINTAINER="${GIRELAY_DEB_MAINTAINER:-}"
STAGE="$ROOT/target/debian-package/girelay_${VERSION}_${ARCH}"
OUTPUT_DIR="$ROOT/dist"
OUTPUT_NAME="girelay_${VERSION}_${ARCH}.deb"
OUTPUT="$OUTPUT_DIR/$OUTPUT_NAME"

case "$ARCH" in
  amd64|arm64) ;;
  *) echo "Unsupported Debian architecture: $ARCH" >&2; exit 2 ;;
esac

if [ "$MODE" = "prepare-only" ]; then
  MAINTAINER="${MAINTAINER:-Girelay release check <release-check@example.invalid>}"
elif [ -z "$MAINTAINER" ]; then
  echo "Set GIRELAY_DEB_MAINTAINER to a real 'Name <email>' identity." >&2
  exit 2
fi

rm -rf "$STAGE"
mkdir -p "$STAGE/DEBIAN" "$STAGE/usr/bin"
sed \
  -e "s/__VERSION__/$VERSION/g" \
  -e "s/__ARCH__/$ARCH/g" \
  -e "s#__MAINTAINER__#$MAINTAINER#g" \
  "$ROOT/packaging/debian/control.template" > "$STAGE/DEBIAN/control"

if [ "$MODE" = "prepare-only" ]; then
  grep -F "Package: girelay" "$STAGE/DEBIAN/control" >/dev/null
  grep -F "Architecture: $ARCH" "$STAGE/DEBIAN/control" >/dev/null
  echo "Prepared Debian metadata at $STAGE/DEBIAN/control"
  exit 0
fi

command -v dpkg-deb >/dev/null 2>&1 || {
  echo "dpkg-deb is required to build a Debian package." >&2
  exit 3
}

cp "$ROOT/target/$TARGET/release/girelay" "$STAGE/usr/bin/girelay"
chmod 0755 "$STAGE/usr/bin/girelay"
mkdir -p "$OUTPUT_DIR"
dpkg-deb --root-owner-group --build "$STAGE" "$OUTPUT"

(
  cd "$OUTPUT_DIR"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$OUTPUT_NAME" > "$OUTPUT_NAME.sha256"
    sha256sum -c "$OUTPUT_NAME.sha256"
  else
    shasum -a 256 "$OUTPUT_NAME" > "$OUTPUT_NAME.sha256"
    shasum -a 256 -c "$OUTPUT_NAME.sha256"
  fi
  test "$(awk 'NR == 1 { print $2 }' "$OUTPUT_NAME.sha256")" = "$OUTPUT_NAME"
)

echo "Wrote $OUTPUT"
echo "Wrote $OUTPUT.sha256"

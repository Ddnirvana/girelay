#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/assets/demo"
RAW="$OUT/girelay-run-demo-raw.txt"
TRANSCRIPT="$OUT/girelay-run-demo-transcript.txt"
MP4="$OUT/girelay-run-demo.mp4"
GIF="$OUT/girelay-run-demo.gif"
NODE="${GIRELAY_NODE:-node}"

command -v ffmpeg >/dev/null 2>&1 || {
  echo "ffmpeg is required to render demo media" >&2
  exit 3
}

mkdir -p "$OUT"
cargo build --workspace >/dev/null
PATH="$ROOT/target/debug:$PATH" bash "$ROOT/scripts/demo.sh" > "$RAW" 2>&1
sed -E \
  -e 's#(/private)?/var/folders/[^ ]*/tmp\.[^ /]*#<demo>#g' \
  -e 's#/tmp/tmp\.[^ /]*#<demo>#g' \
  "$RAW" > "$TRANSCRIPT"
rm -f "$RAW"

command -v "$NODE" >/dev/null 2>&1 || {
  echo "Node.js runtime not found; install Node.js or set GIRELAY_NODE" >&2
  exit 3
}
if [[ -n "${GIRELAY_NODE_MODULES:-}" ]]; then
  NODE_PATH="$GIRELAY_NODE_MODULES" "$NODE" "$ROOT/scripts/render-media.cjs" >/dev/null
else
  "$NODE" "$ROOT/scripts/render-media.cjs" >/dev/null
fi

ffmpeg -y -framerate 1/20 -i "$ROOT/target/demo-frames/frame-%02d.png" \
  -c:v libx264 -pix_fmt yuv420p -r 30 -movflags +faststart "$MP4" >/dev/null 2>&1

ffmpeg -y -i "$MP4" -vf "fps=10,scale=960:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse=dither=bayer" "$GIF" >/dev/null 2>&1

echo "Rendered $MP4"
echo "Rendered $GIF"

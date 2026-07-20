#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TRANSCRIPT="$ROOT/assets/demo/multi-agent-relay-transcript.txt"
FRAMES="$ROOT/target/multi-agent-demo-frames"
MP4="$ROOT/assets/demo/multi-agent-relay.mp4"
GIF="$ROOT/assets/demo/multi-agent-relay.gif"
NODE="${GIRELAY_NODE:-node}"

test -f "$TRANSCRIPT" || {
  echo "record and review $TRANSCRIPT before rendering" >&2
  exit 3
}
command -v ffmpeg >/dev/null 2>&1 || {
  echo "ffmpeg is required to render demo media" >&2
  exit 3
}

if [[ -n "${GIRELAY_NODE_MODULES:-}" ]]; then
  NODE_PATH="$GIRELAY_NODE_MODULES" \
  GIRELAY_TRANSCRIPT="$TRANSCRIPT" \
  GIRELAY_FRAME_DIR="$FRAMES" \
  GIRELAY_DEMO_LABEL="authenticated Codex to Claude relay" \
  GIRELAY_SKIP_SOCIAL=1 \
    "$NODE" "$ROOT/scripts/render-media.cjs" >/dev/null
else
  GIRELAY_TRANSCRIPT="$TRANSCRIPT" \
  GIRELAY_FRAME_DIR="$FRAMES" \
  GIRELAY_DEMO_LABEL="authenticated Codex to Claude relay" \
  GIRELAY_SKIP_SOCIAL=1 \
    "$NODE" "$ROOT/scripts/render-media.cjs" >/dev/null
fi

ffmpeg -y -framerate 1/18 -i "$FRAMES/frame-%02d.png" \
  -c:v libx264 -pix_fmt yuv420p -r 30 -movflags +faststart "$MP4" >/dev/null 2>&1
ffmpeg -y -i "$MP4" \
  -vf "fps=10,scale=960:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse=dither=bayer" \
  "$GIF" >/dev/null 2>&1

echo "Rendered $MP4"
echo "Rendered $GIF"

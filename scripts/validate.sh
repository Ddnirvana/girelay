#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== toolchain =="
rustc --version
cargo --version
cargo clippy --version
echo "== cargo fmt --check =="
cargo fmt --check
echo "== cargo clippy --all-targets -- -D warnings =="
cargo clippy --all-targets -- -D warnings
echo "== cargo test =="
cargo test
echo "== documentation links =="
node scripts/check-links.cjs
echo "== public surface =="
bash scripts/public-surface-check.sh
echo "== agent integration artifacts =="
bash -n \
  scripts/agent-matrix.sh \
  scripts/agent-live-matrix.sh \
  scripts/record-multi-agent-demo.sh \
  scripts/render-demo-video.sh \
  scripts/render-multi-agent-demo.sh
PYTHONPYCACHEPREFIX="$ROOT/target/python-cache" python3 -m py_compile scripts/run-with-timeout.py
node --check scripts/render-media.cjs
for artifact in \
  assets/demo/multi-agent-relay-transcript.txt \
  assets/demo/multi-agent-relay-tests.txt \
  assets/demo/multi-agent-relay.gif \
  assets/demo/multi-agent-relay.mp4 \
  docs/evidence/pi-v2-live-2026-07-20.json \
  docs/evidence/multi-agent-relay-2026-07-20.json; do
  test -s "$artifact"
done
rg -q 'Codex codex-cli 0\.144\.3 -> Claude Code 2\.1\.215' \
  assets/demo/multi-agent-relay-transcript.txt
rg -q 'Restored source branch main to <baseline-commit>' \
  assets/demo/multi-agent-relay-transcript.txt
rg -q 'Ran 3 tests' assets/demo/multi-agent-relay-tests.txt
node -e 'for (const file of process.argv.slice(1)) JSON.parse(require("fs").readFileSync(file, "utf8"))' \
  docs/evidence/pi-v2-live-2026-07-20.json \
  docs/evidence/multi-agent-relay-2026-07-20.json
credential_pattern='s''k-[A-Za-z0-9_-]{12,}'
user_path_pattern='/''Users/'
if rg -n "$user_path_pattern|/private/var/|/tmp/|$credential_pattern" \
  assets/demo/multi-agent-relay-transcript.txt \
  assets/demo/multi-agent-relay-tests.txt \
  docs/evidence/pi-v2-live-2026-07-20.* \
  docs/evidence/multi-agent-relay-2026-07-20.*; then
  echo "publishable agent evidence contains a private path or credential pattern" >&2
  exit 1
fi
echo "== cargo build --workspace =="
cargo build --workspace
echo "== focused lifecycle demo =="
PATH="$ROOT/target/debug:$PATH" bash scripts/demo.sh
echo "== basic example =="
PATH="$ROOT/target/debug:$PATH" bash examples/basic-agent-flow/run.sh
echo "== parallel and relay example =="
PATH="$ROOT/target/debug:$PATH" bash examples/multi-agent-flow/run.sh
echo "== deterministic agent matrix =="
PATH="$ROOT/target/debug:$PATH" bash scripts/agent-matrix.sh
echo "== dogfood temp-copy scenario =="
PATH="$ROOT/target/debug:$PATH" bash scripts/dogfood.sh
echo "girelay validation passed"

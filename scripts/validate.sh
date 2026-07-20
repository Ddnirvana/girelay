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
node scripts/check-agent-artifacts.cjs
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

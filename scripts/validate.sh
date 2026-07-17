#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

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

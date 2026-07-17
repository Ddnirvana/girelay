#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
INSTALL_ROOT="$(mktemp -d)/install"

echo "== validate =="
bash scripts/validate.sh

echo
echo "== version and changelog consistency =="
bash scripts/version-check.sh

echo
echo "== Debian package metadata =="
bash scripts/package-deb.sh prepare-only x86_64-unknown-linux-gnu amd64
bash scripts/package-deb.sh prepare-only aarch64-unknown-linux-gnu arm64

echo
echo "== cargo build --release =="
cargo build --release

echo
echo "== cargo install --path crates/girelay =="
cargo install --path crates/girelay --root "$INSTALL_ROOT" --force --offline

echo
echo "== cargo package =="
bash scripts/package-check.sh

echo
echo "== binary help checks =="
target/release/girelay --help >/dev/null
target/release/girelay setup --help >/dev/null
target/release/girelay start --help >/dev/null
target/release/girelay relay --help >/dev/null
target/release/girelay status --help >/dev/null
target/release/girelay merge --help >/dev/null
target/release/girelay clean --help >/dev/null
target/release/girelay recover --help >/dev/null
"$INSTALL_ROOT/bin/girelay" --version >/dev/null

echo
echo "release check passed"

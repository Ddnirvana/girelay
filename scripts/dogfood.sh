#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
COPY="$TMP/girelay"
rsync -a --exclude .git --exclude target "$ROOT/" "$COPY/"

cd "$COPY"
git init -b main >/dev/null
git config user.email dogfood@example.com
git config user.name "Girelay Dogfood"
git add .
git commit -m "dogfood baseline" >/dev/null

girelay start dogfood-docs --intent "Dogfood the focused lifecycle" -- \
  sh -c 'printf "dogfood\n" > DOGFOOD_RUN.md'
girelay relay dogfood-docs -- sh -c 'test -f DOGFOOD_RUN.md'
girelay status dogfood-docs --json >/dev/null
girelay merge dogfood-docs --strategy squash --message "test: dogfood girelay"
girelay clean dogfood-docs
test -f DOGFOOD_RUN.md
test ! -d .girelay/workspaces/dogfood-docs

echo "dogfood scenario passed"

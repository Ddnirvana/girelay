#!/usr/bin/env bash
set -euo pipefail

ROOT="$(mktemp -d)"
REPO="$ROOT/repo"
mkdir -p "$REPO"
cd "$REPO"
git init -b main >/dev/null
git config user.email "agent@example.com"
git config user.name "Agent Example"
printf '# example\n' > README.md
git add README.md
git commit -m "initial commit" >/dev/null

girelay start docs --intent "Add usage notes" -- sh -c 'printf "usage\n" > USAGE.md'
girelay start code --intent "Add code and tests" -- sh -c 'printf "code\n" > code.txt'
girelay relay code -- sh -c 'printf "tests\n" > tests.txt'
girelay status

girelay merge docs --message "docs: add usage notes"
girelay clean docs
girelay merge code --message "feat: add code and tests"
girelay clean code

git log --oneline --decorate -3

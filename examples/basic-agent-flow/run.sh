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

girelay start greet --intent "Add a greeting module" -- \
  sh -c 'printf "pub fn greeting() -> &\x27static str { \x5c\"hello\x5c\" }\n" > greeting.rs'
girelay status greet
git -C "$REPO/.girelay/workspaces/greet" diff -- greeting.rs
girelay merge greet --strategy squash --message "feat: add greeting"
girelay clean greet

printf '\nFinal source commit:\n'
git log -1 --pretty=medium

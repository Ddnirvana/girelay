#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/target/agent-matrix"
TMP="$(mktemp -d)"
REPO="$TMP/repo"
mkdir -p "$OUT" "$REPO"

cd "$REPO"
git init -b main >/dev/null
git config user.email matrix@example.com
git config user.name "Girelay Matrix"
printf '# matrix\n' > README.md
git add README.md
git commit -m baseline >/dev/null

girelay setup codex --local >/dev/null
girelay setup claude --local >/dev/null
girelay setup pi --local >/dev/null
test -f .girelay/skills/codex/SKILL.md
test -f .girelay/skills/claude/SKILL.md
test -f .girelay/skills/pi/SKILL.md
for skill in .girelay/skills/{codex,claude,pi}/SKILL.md; do
  grep -q 'Read `GIRELAY_INTENT`' "$skill"
  grep -q 'verify its claims against current files' "$skill"
  grep -q 'including a blocker or partial result' "$skill"
  grep -q 'still submit the report' "$skill"
done
girelay start generic --intent "Verify generic agent lifecycle" -- sh -c 'printf "one\n" > one.txt'
girelay relay generic -- sh -c 'printf "two\n" > two.txt'
girelay merge generic --message "test: generic lifecycle"
girelay clean generic

codex_status="unavailable"
claude_status="unavailable"
pi_status="unavailable"
command -v codex >/dev/null 2>&1 && codex_status="cli-present-not-invoked"
command -v claude >/dev/null 2>&1 && claude_status="cli-present-not-invoked"
command -v pi >/dev/null 2>&1 && pi_status="cli-present-not-invoked"

cat > "$OUT/report.json" <<EOF
{
  "schema_version": 2,
  "generic_shell": "verified-live",
  "codex_skill": "artifacts-validated",
  "claude_skill": "artifacts-validated",
  "pi_skill": "artifacts-validated",
  "codex_cli": "$codex_status",
  "claude_cli": "$claude_status",
  "pi_cli": "$pi_status"
}
EOF

cat > "$OUT/report.md" <<EOF
# Agent Matrix

| Runtime | Evidence |
| --- | --- |
| Generic shell | verified-live: start, relay, merge, clean |
| Codex skill | artifacts-validated |
| Claude skill | artifacts-validated |
| Pi skill | artifacts-validated |
| Codex CLI | $codex_status |
| Claude CLI | $claude_status |
| Pi CLI | $pi_status |
EOF

echo "agent matrix passed: $OUT/report.json"

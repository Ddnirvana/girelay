#!/usr/bin/env bash
set -euo pipefail

ROOT="$(mktemp -d)"
REPO="$ROOT/repo"
mkdir -p "$REPO"
cd "$REPO"
git init -b main >/dev/null
git config user.email "agent@example.com"
git config user.name "Agent Demo"
printf '# demo\n' > README.md
git add README.md
git commit -m "initial commit" >/dev/null

echo '$ girelay start parser-fix --intent "Add parser and tests" -- <agent>'
girelay start parser-fix --intent "Add parser and tests" -- \
  sh -c '
    printf "parser\n" > parser.txt
    report="${TMPDIR:-/tmp}/girelay-demo-$GIRELAY_SESSION_ID.json"
    printf '\''{"schema_version":2,"task_id":"%s","session_id":"%s","agent":"sh","start_snapshot":"%s","end_snapshot":null,"summary":"Parser implementation is complete; focused tests remain.","completed":["implemented parser"],"remaining":["add focused tests"],"decisions":["kept parser API stable"],"failed_attempts":[],"blockers":[],"tests":[],"risks":["tests not added yet"],"next_action":"add focused parser tests","trust":"reported-by-agent"}'\'' "$GIRELAY_TASK_ID" "$GIRELAY_SESSION_ID" "$GIRELAY_START_SNAPSHOT" > "$report"
    girelay report --session "$GIRELAY_SESSION_ID" --file "$report"
    rm -f "$report"
  '

echo '$ girelay relay parser-fix -- <next-agent>'
girelay relay parser-fix -- sh -c '
  test -f "$GIRELAY_PREVIOUS_REPORT"
  grep -q "focused tests remain" "$GIRELAY_PREVIOUS_REPORT"
  printf "tests\n" > parser-tests.txt
  report="${TMPDIR:-/tmp}/girelay-demo-$GIRELAY_SESSION_ID.json"
  printf '\''{"schema_version":2,"task_id":"%s","session_id":"%s","agent":"sh","start_snapshot":"%s","end_snapshot":null,"summary":"Parser and focused tests are ready for review.","completed":["verified prior report","added focused tests"],"remaining":[],"decisions":[],"failed_attempts":[],"blockers":[],"tests":["deterministic fixture check"],"risks":[],"next_action":"review and merge","trust":"reported-by-agent"}'\'' "$GIRELAY_TASK_ID" "$GIRELAY_SESSION_ID" "$GIRELAY_START_SNAPSHOT" > "$report"
  girelay report --session "$GIRELAY_SESSION_ID" --file "$report"
  rm -f "$report"
'

echo '$ girelay status parser-fix'
girelay status parser-fix

echo '$ girelay merge parser-fix --strategy squash'
girelay merge parser-fix --strategy squash --message "feat: add parser and tests"

echo '$ girelay clean parser-fix'
girelay clean parser-fix

echo '$ git log -1 --oneline'
git log -1 --oneline

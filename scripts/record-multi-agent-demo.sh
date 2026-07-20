#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/target/multi-agent-demo"
RUN="$(mktemp -d)"
REPO="$RUN/repo"
PI_HOME="$RUN/pi-home"
PI_CONFIG="$PI_HOME/.pi/agent"
RAW="$OUT/raw-agent-output.txt"
TRANSCRIPT="$OUT/transcript.txt"
SECOND_AGENT="${GIRELAY_DEMO_SECOND_AGENT:-claude}"
SECOND_MODEL="${GIRELAY_DEMO_MODEL:-gpt-5.6-luna}"
CLAUDE_BIN="${GIRELAY_CLAUDE_BIN:-claude}"
CLAUDE_BASE_URL="${GIRELAY_CLAUDE_BASE_URL:-http://localhost:3000}"
CLAUDE_API_KEY="${GIRELAY_CLAUDE_API_KEY:-${GIRELAY_PI_API_KEY:-}}"
PI_BASE_URL="${GIRELAY_PI_BASE_URL:-http://localhost:3000/v1}"
AGENT_TIMEOUT="${GIRELAY_DEMO_AGENT_TIMEOUT_SECONDS:-600}"

for command in girelay codex git python3; do
  command -v "$command" >/dev/null 2>&1 || {
    echo "$command is required for the authenticated relay demo" >&2
    exit 3
  }
done
case "$SECOND_AGENT" in
  claude)
    SECOND_LABEL="Claude Code"
    test -x "$CLAUDE_BIN" || command -v "$CLAUDE_BIN" >/dev/null 2>&1 || {
      echo "Claude Code is required for the default authenticated relay demo" >&2
      exit 3
    }
    : "${CLAUDE_API_KEY:?set GIRELAY_CLAUDE_API_KEY for the Claude endpoint}"
    ;;
  pi)
    SECOND_LABEL="Pi"
    command -v pi >/dev/null 2>&1 || { echo "Pi is required for the selected demo" >&2; exit 3; }
    : "${GIRELAY_PI_API_KEY:?set GIRELAY_PI_API_KEY for the Pi provider}"
    ;;
  *)
    echo "GIRELAY_DEMO_SECOND_AGENT must be claude or pi" >&2
    exit 3
    ;;
esac

rm -rf "$OUT"
mkdir -p "$OUT" "$REPO" "$PI_CONFIG"
cp -R "$ROOT/examples/defining-relay-demo/." "$REPO/"
git -C "$REPO" init -b main >/dev/null
git -C "$REPO" config user.email relay-demo@example.com
git -C "$REPO" config user.name "Girelay Relay Demo"
git -C "$REPO" add .
git -C "$REPO" commit -m baseline >/dev/null
BASE_COMMIT="$(git -C "$REPO" rev-parse HEAD)"

girelay setup codex >/dev/null
if [[ "$SECOND_AGENT" == "claude" ]]; then
  girelay setup claude >/dev/null
else
  HOME="$PI_HOME" girelay setup pi >/dev/null
  cat > "$PI_CONFIG/models.json" <<JSON
{
  "providers": {
    "girelay-live": {
      "baseUrl": "$PI_BASE_URL",
      "api": "openai-responses",
      "apiKey": "\$GIRELAY_PI_API_KEY",
      "models": [{
        "id": "$SECOND_MODEL",
        "name": "$SECOND_MODEL",
        "reasoning": true,
        "input": ["text"],
        "contextWindow": 200000,
        "maxTokens": 32000,
        "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0}
      }]
    }
  }
}
JSON
fi

CODEX_PROMPT="Read and follow the installed girelay skill. Implement only stage one: make tests/test_normalization.py pass by fixing task_id.py. Do not implement reserved Git-name rejection. Run python3 -m unittest tests.test_normalization -v. Submit a semantic report that explicitly lists reserved Git-name rejection and the full suite as remaining work for the next agent."
SECOND_PROMPT="Read and follow the installed girelay skill. Read and verify GIRELAY_PREVIOUS_REPORT against the current diff and tests. Complete the remaining reserved Git-name rejection in task_id.py. Change no other tracked file. Run python3 -m unittest discover -s tests -v and submit the final semantic report."

cd "$REPO"
{
  echo '$ girelay start task-id-safety -- codex'
  python3 "$ROOT/scripts/run-with-timeout.py" "$AGENT_TIMEOUT" \
    girelay start task-id-safety --intent "Normalize task ids and reject reserved Git names" -- \
      codex exec --sandbox workspace-write --ephemeral "$CODEX_PROMPT"
  echo "$ girelay relay task-id-safety -- $SECOND_AGENT"
  if [[ "$SECOND_AGENT" == "claude" ]]; then
    python3 "$ROOT/scripts/run-with-timeout.py" "$AGENT_TIMEOUT" \
      env ANTHROPIC_API_KEY="$CLAUDE_API_KEY" ANTHROPIC_BASE_URL="$CLAUDE_BASE_URL" \
      girelay relay task-id-safety -- \
        "$CLAUDE_BIN" --bare --model "$SECOND_MODEL" --dangerously-skip-permissions \
        --no-session-persistence --print "$SECOND_PROMPT"
  else
    python3 "$ROOT/scripts/run-with-timeout.py" "$AGENT_TIMEOUT" \
      env HOME="$PI_HOME" PI_CODING_AGENT_DIR="$PI_CONFIG" \
      girelay relay task-id-safety -- \
        pi --provider girelay-live --model "$SECOND_MODEL" --no-session --print "$SECOND_PROMPT"
  fi
} > "$RAW" 2>&1

TASK_JSON="$REPO/.girelay/tasks/task-id-safety.json"
SECOND_SESSION="$(sed -n 's/.*"latest_session_id": "\([^"]*\)".*/\1/p' "$TASK_JSON")"
CODEX_SESSION="$(find "$REPO/.girelay/reports/task-id-safety" -type f -name '*.json' \
  ! -name "$SECOND_SESSION.json" -exec basename {} .json \;)"
test -n "$CODEX_SESSION"
test -n "$SECOND_SESSION"
CODEX_REPORT="$REPO/.girelay/reports/task-id-safety/$CODEX_SESSION.json"
SECOND_REPORT="$REPO/.girelay/reports/task-id-safety/$SECOND_SESSION.json"
test -f "$CODEX_REPORT"
test -f "$SECOND_REPORT"
grep -q 'reserved Git' "$CODEX_REPORT"
grep -q 'tests.test_normalization' "$CODEX_REPORT"
grep -Eqi 'previous[^".]*report' "$SECOND_REPORT"
grep -q "\"agent\": \"$SECOND_AGENT\"" "$SECOND_REPORT"

WORKSPACE="$REPO/.girelay/workspaces/task-id-safety"
(cd "$WORKSPACE" && python3 -m unittest discover -s tests -v) > "$OUT/tests.txt" 2>&1
test "$(git -C "$WORKSPACE" status --porcelain | sed -E 's/^...//' | sort)" = "task_id.py"
test -z "$(git -C "$REPO" status --short)"

{
  echo '# girelay authenticated multi-agent relay'
  echo
  if [[ "$SECOND_AGENT" == "claude" ]]; then
    echo "Agents: Codex $(codex --version 2>/dev/null | tail -n 1) -> Claude Code $("$CLAUDE_BIN" --version | awk '{print $1}')"
  else
    echo "Agents: Codex $(codex --version 2>/dev/null | tail -n 1) -> Pi $(pi --version | head -n 1)"
  fi
  echo "Second-agent model: $SECOND_MODEL"
  echo 'Fixture: examples/defining-relay-demo'
  echo
  echo '$ girelay start task-id-safety -- codex'
  echo 'Codex completed normalization, ran focused tests, and reported reserved Git names as remaining.'
  echo "$ girelay relay task-id-safety -- $SECOND_AGENT"
  echo "$SECOND_LABEL read the previous report, verified stage one, completed reserved-name rejection, and ran the full suite."
  echo
  echo '$ girelay status task-id-safety'
  girelay status task-id-safety
  echo
  echo '$ girelay merge task-id-safety --dry-run'
  girelay merge task-id-safety --message "fix: normalize safe task ids" --dry-run
} > "$TRANSCRIPT"

girelay merge task-id-safety --message "fix: normalize safe task ids" --json > "$OUT/merge.json"
TASK_REF="$(sed -n 's/.*"task_rollback_ref": "\([^"]*\)".*/\1/p' "$OUT/merge.json")"
SOURCE_REF="$(sed -n 's/.*"source_rollback_ref": "\([^"]*\)".*/\1/p' "$OUT/merge.json")"
git show-ref --verify --quiet "$TASK_REF"
git show-ref --verify --quiet "$SOURCE_REF"
MERGED_COMMIT="$(git rev-parse HEAD)"
girelay recover list task-id-safety --json > "$OUT/recovery.json"
RECOVERY_COUNT="$(sed -n 's/.*"count": \([0-9]*\).*/\1/p' "$OUT/recovery.json" | head -n 1)"
{
  echo
  echo '$ girelay merge task-id-safety --message "fix: normalize safe task ids"'
  echo 'Merged task task-id-safety with squash strategy'
  echo 'Source commit: <merged-task-commit>'
  echo 'Rollback: refs/girelay/rollback/source/task-id-safety/<merge-id>'
  echo '$ girelay recover list task-id-safety'
  echo "Recovery points: $RECOVERY_COUNT, including both session snapshots and merge rollback refs."
  echo '$ girelay clean task-id-safety'
} >> "$TRANSCRIPT"
girelay clean task-id-safety >/dev/null
git show-ref --verify --quiet refs/heads/agent/task-id-safety
{
  echo 'Cleaned task task-id-safety'
  echo 'Branch: retained'
  echo '$ girelay recover restore refs/girelay/rollback/source/task-id-safety/<merge-id> --confirm'
} >> "$TRANSCRIPT"
girelay recover restore "$SOURCE_REF" --confirm >> "$TRANSCRIPT"
test "$(git rev-parse HEAD)" = "$BASE_COMMIT"
test "$MERGED_COMMIT" != "$BASE_COMMIT"
test -z "$(git status --short)"

cp "$CODEX_REPORT" "$OUT/codex-report.json"
cp "$SECOND_REPORT" "$OUT/$SECOND_AGENT-report.json"
{
  echo
  echo '$ git status --short --branch'
  echo '## main'
  echo '$ git rev-parse HEAD'
  echo '<baseline restored>'
  echo '$ git show-ref --verify refs/heads/agent/task-id-safety'
  echo '<task branch retained>'
} >> "$TRANSCRIPT"

python3 - "$OUT" "$REPO" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
repo = sys.argv[2]
for path in root.glob("*.json"):
    value = json.loads(path.read_text())
    def clean(item):
        if isinstance(item, dict):
            return {key: clean(child) for key, child in item.items()}
        if isinstance(item, list):
            return [clean(child) for child in item]
        if isinstance(item, str):
            return item.replace(repo, "<fixture-repo>")
        return item
    path.write_text(json.dumps(clean(value), indent=2) + "\n")
PY

sed -E \
  -e 's#(/private)?/var/folders/[^ ]*/tmp\.[^ /]*#<demo>#g' \
  -e 's#/tmp/tmp\.[^ /]*#<demo>#g' \
  -e 's/(Restored source branch main to) [0-9a-f]{40}/\1 <baseline-commit>/' \
  "$TRANSCRIPT" > "$OUT/transcript.sanitized.txt"
mv "$OUT/transcript.sanitized.txt" "$TRANSCRIPT"
if [[ -n "$CLAUDE_API_KEY" ]] && grep -R -F -- "$CLAUDE_API_KEY" "$OUT" >/dev/null; then
  echo "credential leaked into demo artifacts" >&2
  exit 4
fi
credential_pattern='s''k-[A-Za-z0-9_-]{12,}'
user_path_pattern='/''Users/'
if rg -n "$user_path_pattern|/private/var/|/tmp/|$credential_pattern" "$TRANSCRIPT" "$OUT"/*.json; then
  echo "private path or credential pattern remains in publishable demo artifacts" >&2
  exit 4
fi

echo "authenticated relay demo passed: $OUT"

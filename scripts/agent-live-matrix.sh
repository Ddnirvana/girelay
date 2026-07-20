#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SELECTED=",${GIRELAY_LIVE_AGENTS:-},"
OUT="$ROOT/target/agent-live-matrix"
mkdir -p "$OUT"

if [[ "$SELECTED" == ",," ]]; then
  echo "Set GIRELAY_LIVE_AGENTS=codex,claude,pi to opt into authenticated model runs." >&2
  exit 2
fi

make_fixture() {
  local repo="$1"
  mkdir -p "$repo"
  cp -R "$ROOT/examples/agent-live-fixture/." "$repo/"
  git -C "$repo" init -b main >/dev/null
  git -C "$repo" config user.email live-matrix@example.com
  git -C "$repo" config user.name "Girelay Live Matrix"
  git -C "$repo" add .
  git -C "$repo" commit -m baseline >/dev/null
}

sanitize_json_paths() {
  local file="$1" repo="$2"
  python3 - "$file" "$repo" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
repo = sys.argv[2]

def sanitize(value):
    if isinstance(value, dict):
        return {key: sanitize(item) for key, item in value.items()}
    if isinstance(value, list):
        return [sanitize(item) for item in value]
    if isinstance(value, str):
        return value.replace(repo, "<fixture-repo>")
    return value

path.write_text(json.dumps(sanitize(json.loads(path.read_text())), indent=2) + "\n")
PY
}

verify_and_merge() {
  local repo="$1" task="$2" agent="$3"
  local workspace="$repo/.girelay/workspaces/$task"
  (cd "$workspace" && python3 -m unittest discover -s tests -v) \
    > "$OUT/$agent-tests.txt" 2>&1
  test -z "$(git -C "$repo" status --short)"
  test "$(git -C "$workspace" status --porcelain | sed -E 's/^...//' | sort)" = "task_id.py"
  local task_json="$repo/.girelay/tasks/$task.json"
  local session_id report task_ref source_ref
  session_id="$(sed -n 's/.*"latest_session_id": "\([^"]*\)".*/\1/p' "$task_json")"
  test -n "$session_id"
  report="$repo/.girelay/reports/$task/$session_id.json"
  test -f "$report"
  cp "$report" "$OUT/$agent-report.json"
  grep -q 'python3 -m unittest discover -s tests -v -> passed' "$report"
  (cd "$repo" && girelay status "$task" --json > "$OUT/$agent-status.json")
  sanitize_json_paths "$OUT/$agent-status.json" "$repo"
  (cd "$repo" && girelay merge "$task" --message "fix: normalize task ids" --dry-run --json \
    > "$OUT/$agent-preview.json")
  (cd "$repo" && girelay merge "$task" --message "fix: normalize task ids" --json > "$OUT/$agent-merge.json")
  test -f "$repo/task_id.py"
  task_ref="$(sed -n 's/.*"task_rollback_ref": "\([^"]*\)".*/\1/p' "$OUT/$agent-merge.json")"
  source_ref="$(sed -n 's/.*"source_rollback_ref": "\([^"]*\)".*/\1/p' "$OUT/$agent-merge.json")"
  test -n "$task_ref"
  test -n "$source_ref"
  git -C "$repo" show-ref --verify --quiet "$task_ref"
  git -C "$repo" show-ref --verify --quiet "$source_ref"
  (cd "$repo" && girelay recover list "$task" --json > "$OUT/$agent-recovery.json")
  (cd "$repo" && girelay clean "$task")
  git -C "$repo" show-ref --verify --quiet "refs/heads/agent/$task"
  test -z "$(git -C "$repo" status --short)"
  git -C "$repo" show --stat --oneline -1 > "$OUT/$agent-source-commit.txt"
}

run_codex() {
  command -v codex >/dev/null 2>&1 || { echo "codex was selected but is unavailable" >&2; return 3; }
  local repo
  repo="$(mktemp -d)/repo"
  mkdir -p "$repo"
  make_fixture "$repo"
  (cd "$repo" && ! python3 -m unittest discover -s tests -v >/dev/null 2>&1)
  girelay setup codex >/dev/null
  local prompt="Fix task_id.py so all tests pass. Change no other tracked file. Run python3 -m unittest discover -s tests -v. Follow the installed girelay skill and submit its semantic report before exiting."
  (cd "$repo" && girelay start codex-live --intent "Normalize task ids and pass all tests" -- \
    codex exec --sandbox workspace-write "$prompt")
  verify_and_merge "$repo" codex-live codex
  printf '{"schema_version":2,"agent":"codex","status":"authenticated-live","task":"codex-live"}\n' > "$OUT/codex.json"
}

run_claude() {
  local binary="${GIRELAY_CLAUDE_BIN:-claude}"
  test -x "$binary" || command -v "$binary" >/dev/null 2>&1 || {
    echo "claude was selected but is unavailable" >&2
    return 3
  }
  local repo version
  repo="$(mktemp -d)/repo"
  mkdir -p "$repo"
  make_fixture "$repo"
  (cd "$repo" && ! python3 -m unittest discover -s tests -v >/dev/null 2>&1)
  girelay setup claude >/dev/null
  local prompt="Fix task_id.py so all tests pass. Change no other tracked file. Run python3 -m unittest discover -s tests -v. Follow the installed girelay skill and submit its semantic report before exiting."
  (cd "$repo" && girelay start claude-live --intent "Normalize task ids and pass all tests" -- \
    "$binary" -p "$prompt")
  verify_and_merge "$repo" claude-live claude
  version="$("$binary" --version | head -n 1)"
  printf '{"schema_version":2,"agent":"claude","agent_version":"%s","status":"authenticated-live","task":"claude-live"}\n' \
    "$version" > "$OUT/claude.json"
}

run_pi() {
  command -v pi >/dev/null 2>&1 || { echo "pi was selected but is unavailable" >&2; return 3; }
  : "${GIRELAY_PI_API_KEY:?set GIRELAY_PI_API_KEY for the selected Pi provider}"
  local base_url="${GIRELAY_PI_BASE_URL:-http://localhost:3000/v1}"
  local model="${GIRELAY_PI_MODEL:-gpt-5.6-luna}"
  local root repo home config prompt version
  root="$(mktemp -d)"
  repo="$root/repo"
  home="$root/home"
  config="$home/.pi/agent"
  mkdir -p "$repo" "$config"
  make_fixture "$repo"
  (cd "$repo" && ! python3 -m unittest discover -s tests -v >/dev/null 2>&1)
  HOME="$home" girelay setup pi >/dev/null
  cat > "$config/models.json" <<JSON
{
  "providers": {
    "girelay-live": {
      "baseUrl": "$base_url",
      "api": "openai-responses",
      "apiKey": "\$GIRELAY_PI_API_KEY",
      "models": [{
        "id": "$model",
        "name": "$model",
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
  prompt="Read and follow the installed girelay skill. Fix task_id.py so all tests pass. Change no other tracked file. Run python3 -m unittest discover -s tests -v. Submit the required semantic report before exiting."
  (cd "$repo" && HOME="$home" PI_CODING_AGENT_DIR="$config" \
    girelay start pi-live --intent "Normalize task ids and pass all tests" -- \
    pi --provider girelay-live --model "$model" --no-session --print "$prompt")
  verify_and_merge "$repo" pi-live pi
  version="$(pi --version | head -n 1)"
  cat > "$OUT/pi.json" <<EOF
{"schema_version":2,"agent":"pi","agent_version":"$version","model":"$model","status":"authenticated-live","task":"pi-live"}
EOF
  if grep -R -F -- "$GIRELAY_PI_API_KEY" "$OUT" >/dev/null; then
    echo "Pi credential leaked into live evidence" >&2
    return 4
  fi
}

[[ "$SELECTED" == *",codex,"* ]] && run_codex
[[ "$SELECTED" == *",claude,"* ]] && run_claude
[[ "$SELECTED" == *",pi,"* ]] && run_pi

echo "live matrix passed: $OUT"

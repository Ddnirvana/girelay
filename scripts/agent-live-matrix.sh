#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SELECTED=",${GIRELAY_LIVE_AGENTS:-},"
OUT="$ROOT/target/agent-live-matrix"
mkdir -p "$OUT"

if [[ "$SELECTED" == ",," ]]; then
  echo "Set GIRELAY_LIVE_AGENTS=codex,claude to opt into authenticated model runs." >&2
  exit 2
fi

make_fixture() {
  local repo="$1"
  mkdir -p "$repo/tests"
  git -C "$repo" init -b main >/dev/null
  git -C "$repo" config user.email live-matrix@example.com
  git -C "$repo" config user.name "Girelay Live Matrix"
  cat > "$repo/task_id.py" <<'PY'
def normalize_task_id(value: str) -> str:
    return value.replace(" ", "-")
PY
  cat > "$repo/tests/test_task_id.py" <<'PY'
import unittest
from task_id import normalize_task_id

class TaskIdTests(unittest.TestCase):
    def test_lowercases_and_collapses_separators(self):
        self.assertEqual(normalize_task_id(" Auth__ Refresh "), "auth-refresh")

    def test_rejects_empty_result(self):
        with self.assertRaises(ValueError):
            normalize_task_id("___")

if __name__ == "__main__":
    unittest.main()
PY
  printf '# live fixture\n' > "$repo/README.md"
  git -C "$repo" add .
  git -C "$repo" commit -m baseline >/dev/null
}

verify_and_merge() {
  local repo="$1" task="$2" agent="$3"
  local workspace="$repo/.girelay/workspaces/$task"
  (cd "$workspace" && python3 -m unittest discover -s tests -v)
  test -z "$(git -C "$repo" status --short)"
  test "$(git -C "$workspace" status --porcelain | sed -E 's/^...//' | sort)" = "task_id.py"
  local task_json="$repo/.girelay/tasks/$task.json"
  local session_id
  session_id="$(sed -n 's/.*"latest_session_id": "\([^"]*\)".*/\1/p' "$task_json")"
  test -n "$session_id"
  test -f "$repo/.girelay/reports/$task/$session_id.json"
  (cd "$repo" && girelay merge "$task" --message "fix: normalize task ids" --json > "$OUT/$agent-merge.json")
  test -f "$repo/task_id.py"
  (cd "$repo" && girelay clean "$task")
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
  command -v claude >/dev/null 2>&1 || { echo "claude was selected but is unavailable" >&2; return 3; }
  local repo
  repo="$(mktemp -d)/repo"
  mkdir -p "$repo"
  make_fixture "$repo"
  (cd "$repo" && ! python3 -m unittest discover -s tests -v >/dev/null 2>&1)
  girelay setup claude >/dev/null
  local prompt="Fix task_id.py so all tests pass. Change no other tracked file. Run python3 -m unittest discover -s tests -v. Follow the installed girelay skill and submit its semantic report before exiting."
  (cd "$repo" && girelay start claude-live --intent "Normalize task ids and pass all tests" -- \
    claude -p "$prompt")
  verify_and_merge "$repo" claude-live claude
  printf '{"schema_version":2,"agent":"claude","status":"authenticated-live","task":"claude-live"}\n' > "$OUT/claude.json"
}

[[ "$SELECTED" == *",codex,"* ]] && run_codex
[[ "$SELECTED" == *",claude,"* ]] && run_claude

echo "live matrix passed: $OUT"

# Codex v2 Live Evidence - 2026-07-15

## Scope

An authenticated Codex CLI 0.144.3 process ran in a disposable Git repository
through the public girelay v2 lifecycle. No real project repository or remote
was modified.

## Scenario

The fixture contained a Python task-id normalizer and two tests. Both required
behaviors were absent in the baseline. The harness asked Codex to modify only
`task_id.py`, run the complete test suite, follow the installed girelay skill,
and submit a semantic report.

## Observed Evidence

- girelay created a native `agent/codex-live` worktree.
- Codex ran inside that worktree with the durable intent and session variables.
- The source checkout remained clean during the agent session.
- Only `task_id.py` changed.
- The fixture's two tests passed after the agent exited.
- Codex submitted a session-bound semantic report.
- girelay captured start/end snapshots and the successful process result.
- Squash merge changed only `task_id.py` and created task/source rollback refs.
- Cleanup removed the worktree and retained the task branch.

## Protocol Refinement

Codex's first report attempt encoded test evidence as objects. The v2 schema
expects arrays of strings, so girelay rejected the report. Codex corrected the
temporary JSON and resubmitted successfully. The built-in skill was then
strengthened with an exact typed JSON template and an explicit rule that every
list field contains strings.

This rejection is useful evidence: report validation is active, and a malformed
agent claim is not silently persisted.

## Boundaries

- Claude Code was not installed for this July 15 Codex-only run. A later,
  separate [Codex-to-Claude relay](multi-agent-relay-2026-07-20.md) provides
  reviewed Claude Code runtime evidence.
- The fixture was intentionally small and does not prove model quality on large
  repositories.
- Semantic fields remain `reported-by-agent`; the harness independently checked
  changed files and test results.
- Private paths, raw credentials, and unrelated model logs are not published.

Machine-readable summary: [codex-v2-live-2026-07-15.json](codex-v2-live-2026-07-15.json).

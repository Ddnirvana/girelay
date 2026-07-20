# Generic Agent Adapter Contract

girelay can launch any command-line agent without agent-specific Rust code.
Every agent session starts in the task worktree and receives the same seven
environment variables.

## Environment

| Variable | Meaning |
| --- | --- |
| `GIRELAY_TASK_ID` | Stable task identifier chosen at `start`. |
| `GIRELAY_SESSION_ID` | Unique identifier for this one process run. |
| `GIRELAY_INTENT` | Durable task intent, either explicit or the task id. |
| `GIRELAY_SOURCE_REPO` | Absolute path to the clean source checkout that owns the task. |
| `GIRELAY_START_SNAPSHOT` | Commit id of girelay's hidden pre-session snapshot. |
| `GIRELAY_PREVIOUS_REPORT` | Absolute report path for the previous session, or an empty string. |
| `GIRELAY_REPORT_COMMAND` | girelay-generated command prefix that accepts the report file path as its final argument. |

All seven variables are defined for every successfully launched session.
`GIRELAY_PREVIOUS_REPORT` is empty for the first session and whenever the
previous session did not submit a report. An adapter must test for a non-empty
value before reading the path.

Environment values are local coordination data, not credentials. girelay does
not copy model API keys into metadata or reports. Credentials already present
in the parent environment follow normal child-process inheritance.

## Session Rules

An adapter should instruct the agent to:

1. treat `GIRELAY_INTENT` as the task objective;
2. inspect current files, Git status, relevant history, and diffs before editing;
3. read a non-empty `GIRELAY_PREVIOUS_REPORT` and verify every relied-on claim;
4. work only in the current task worktree;
5. avoid merge, push, worktree creation, branch switching, and `.girelay/` edits;
6. submit a final report on success, partial completion, or a blocker.

The child process exit code remains independent from report submission. A
successful exit without a report is recorded as completed with semantic context
`not-reported`; a report never converts a failed process into a successful one.

## Report Protocol

Reports must conform to [`report.schema.json`](../schemas/report.schema.json).
They are JSON objects with:

- identity copied exactly from the environment;
- `end_snapshot` set to `null` because girelay records it after process exit;
- non-empty `summary` and `next_action` strings;
- string arrays for `completed`, `remaining`, `decisions`, `failed_attempts`,
  `blockers`, `tests`, and `risks`;
- trust set exactly to `reported-by-agent`.

Submit from the active task session:

```bash
report="${TMPDIR:-/tmp}/girelay-report-$GIRELAY_SESSION_ID.json"
girelay report --session "$GIRELAY_SESSION_ID" --file "$report"
rm -f "$report"
```

`GIRELAY_REPORT_COMMAND` represents the same girelay-generated command prefix.
Adapters that execute argument arrays should invoke `girelay`, `report`,
`--session`, the session id, `--file`, and the report path as separate arguments
instead of evaluating a shell string.

Submission is accepted only while the matching session is active. girelay
checks task id, session id, agent name, start snapshot, trust label, required
fields, size, and schema version. Reports are immutable after acceptance.

## Trust Boundary

girelay observes process status, timestamps, Git snapshots, paths, commits,
locks, and refs. The report's summaries, test claims, decisions, blockers, and
risks are statements by the agent. A later agent or human must verify them.

An integration claim therefore needs two kinds of evidence:

- deterministic artifact tests showing the adapter receives and can use this
  contract;
- a reviewed live lifecycle showing the real agent edits the fixture, runs the
  expected test, submits a valid report, merges, and cleans successfully.

Binary detection alone is not an integration claim.

# Command Reference

## `girelay setup`

```bash
girelay setup codex
girelay setup claude
girelay setup codex --local
```

The default installs the `girelay` protocol skill in the agent's user-level
skill directory. `--local` writes it under the current repository's excluded
`.girelay/skills/` directory for inspection or manual integration; it does not
modify tracked `AGENTS.md` or `CLAUDE.md` files.

## `girelay start`

```bash
girelay start <task> --intent "<durable intent>"
girelay start <task> --intent "<durable intent>" -- <agent> [args...]
girelay start <task> --intent "<durable intent>" --base <branch> -- <agent>
```

Creates `agent/<task>` and a native worktree at
`.girelay/workspaces/<task>`. With an agent command, it immediately records and
runs the first session. Without one, use `relay` later.

Refusals include invalid/duplicate task ids, dirty source checkout, detached or
missing base, existing task branch, linked-worktree invocation, and conflicting
workspace path.

## `girelay relay`

```bash
girelay relay <task> -- <agent> [args...]
girelay relay <task> --recover-stale-session -- <agent> [args...]
```

Runs another session in the existing task worktree. girelay exports:

```text
GIRELAY_TASK_ID
GIRELAY_SESSION_ID
GIRELAY_INTENT
GIRELAY_SOURCE_REPO
GIRELAY_START_SNAPSHOT
GIRELAY_PREVIOUS_REPORT   # only when one exists
GIRELAY_REPORT_COMMAND
```

The child exit code is propagated. Git state is snapshotted even when the child
fails or cannot start. `--recover-stale-session` removes a task lock only after
you have independently confirmed the previous process is gone.

## `girelay status`

```bash
girelay status
girelay status <task>
girelay status [<task>] --json
```

Lifecycle states are factual:

| State | Meaning |
| --- | --- |
| `created` | Worktree exists; no completed session is recorded. |
| `running` | The task lock exists. |
| `paused` | At least one session finished and the task is not merged. |
| `merged` | A merge record exists. |
| `cleaned` | Cleanup removed the recorded worktree as intended. |
| `missing` | The recorded worktree path is absent. |
| `blocked` | Metadata and lock facts contradict each other. |

Dirty state, changed files, report availability, active/latest session ids,
merge commit, and blockers are separate fields rather than overloaded states.

## `girelay merge`

```bash
girelay merge <task> --strategy squash
girelay merge <task> --strategy preserve
girelay merge <task> --strategy squash --message "fix: clear description"
girelay merge <task> --json
```

Run from the clean source checkout on the task's recorded base branch.

`squash` creates one source commit. `preserve` performs a non-fast-forward Git
merge and retains task commits. If the worktree is dirty, girelay creates one
final task commit after checks pass; hidden relay snapshots never become
intermediate branch commits.

Before integration, girelay:

1. acquires the task lock;
2. captures the complete pre-merge worktree state;
3. runs configured checks unless `--no-checks` is explicit;
4. creates a task rollback ref;
5. finalizes uncommitted task work;
6. revalidates source branch, commit, and cleanliness;
7. creates a source rollback ref;
8. applies the selected merge strategy.

Conflicts restore the source checkout to its original clean commit. girelay
never pushes or opens a pull request.

## `girelay clean`

```bash
girelay clean <task> --dry-run
girelay clean <task> --dry-run --json
girelay clean <task>
girelay clean <task> --archive
girelay clean <task> --delete-branch
```

Default cleanup removes the worktree and retains the task branch. Dirty work is
refused unless `--archive` captures it or `--discard-uncommitted` explicitly
discards it. `--discard-unreachable` acknowledges missing committed
preservation and does not imply dirty-file deletion.

`--delete-branch` requires an unchanged merge record, exact source and task
tips, clean source checkout, matching target branch, and both rollback refs.

## `girelay recover`

```bash
girelay recover list [<task>] [--json]
girelay recover show <recovery-id> [--json]
girelay recover restore <recovery-id> --confirm
```

Recovery ids are printed by `list`. Snapshot and task rollback restoration
creates a fresh `recovery/<task>/...` branch and worktree without overwriting
the task. Source pre-merge restoration requires the source to remain exactly at
the recorded merge result. Cleanup archive restoration verifies SHA-256 and the
Git bundle before recreating the task worktree.

## Internal Report Operation

Installed skills use a hidden `girelay report` operation. It accepts one
session-bound JSON file, validates task/session/agent/start-snapshot identity,
requires non-empty summary and next action, labels the content
`reported-by-agent`, and publishes it atomically. Reports are immutable.

# Troubleshooting

## Source Checkout Is Dirty

`start` and `merge` require a clean source checkout. Commit or stash source
changes; task-worktree dirtiness is handled separately.

## Command Was Run From A Task Worktree

Run `start` and `merge` from the source checkout. `relay`, `status`, `clean`,
and `recover` can locate the source through Git's common directory.

## Task Is Running Or Has A Stale Lock

Do not remove the lock while an agent is alive. After confirming the old parent
and child processes are gone:

```bash
girelay relay <task> --recover-stale-session -- <agent>
```

## Merge Check Failed

The source checkout is unchanged. Run the printed check inside
`.girelay/workspaces/<task>`, fix the issue, and retry.

## Merge Conflict

girelay restores the clean source commit. Resolve source/task divergence using
normal Git inside the task worktree, then retry `merge`.

## Cleanup Refuses Dirty Work

Inspect first:

```bash
girelay clean <task> --dry-run
```

Use `--archive` to preserve the complete file state, or
`--discard-uncommitted` only when loss is intentional.

## Branch Deletion Is Refused

The task was not merged by girelay, source advanced, task branch changed, or a
rollback ref is missing. Keep the branch and review manually; the guard is
designed not to be bypassed.

## Source Recovery Is Stale

After source advances beyond the recorded merge result, use normal `git revert`
or reviewed history repair. girelay intentionally refuses an old hard reset.

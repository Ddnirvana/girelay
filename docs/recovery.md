# Recovery Cookbook

Start by listing factual recovery points:

```bash
girelay recover list
girelay recover list parser-fix --json
girelay recover show <recovery-id>
```

## Agent Process Failed

The child exit code is returned, but the end snapshot and session record are
still captured. Inspect the task and continue it:

```bash
girelay status parser-fix
git -C .girelay/workspaces/parser-fix status
girelay relay parser-fix -- <agent>
```

## Agent Process Was Killed

An abruptly killed parent may leave an active session id and task lock. First
confirm both girelay and the child agent process have stopped. Then:

```bash
girelay relay parser-fix --recover-stale-session -- <agent>
```

The old session is closed as `interrupted` with a recovery-time snapshot; the
new session gets a new id and snapshots.

## Restore A Relay Snapshot

```bash
girelay recover restore \
  refs/girelay/snapshots/parser-fix/<session>/end \
  --confirm
```

This creates a fresh `recovery/parser-fix/...` branch and worktree. It does not
reset or overwrite the task branch.

## Undo The Latest Recorded Merge

Use the exact source rollback id printed by `merge`:

```bash
girelay recover show refs/girelay/rollback/source/parser-fix/<id>
girelay recover restore refs/girelay/rollback/source/parser-fix/<id> --confirm
```

The operation succeeds only when source is clean, on the recorded target, and
still exactly at the recorded merged commit. After source advances, stale
rollback is refused; use normal Git review and revert instead.

## Restore A Cleanup Archive

```bash
girelay recover restore archive/parser-fix-<id> --confirm
```

girelay verifies the manifest SHA-256 and Git bundle, rejects an unrelated or
divergent task branch, recreates the worktree, and restores archived dirty state
as uncommitted work. The archive remains available.

## Merge Conflict

girelay restores the clean source checkout automatically. The task branch and
source/task rollback refs remain for inspection. Resolve the divergence in the
task worktree or source history, then retry `merge`; do not delete refs merely
to bypass the refusal.

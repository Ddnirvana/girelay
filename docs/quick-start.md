# Quick Start

Build or install girelay, then enter a clean Git checkout on `main`.

```bash
cargo install --path crates/girelay
girelay setup codex
```

Create a task and launch the agent in one command:

```bash
girelay start login-fix -- codex
```

Without `--intent`, the exact task id (`login-fix`) becomes the durable intent.
For a more precise handoff, supply an explicit optional intent:

```bash
girelay start login-fix \
  --intent "Fix the flaky login timeout and run focused tests" \
  -- codex
```

girelay creates:

```text
branch:    agent/login-fix
worktree:  .girelay/workspaces/login-fix
metadata:  .girelay/tasks/login-fix.json
```

The `.girelay/` directory is excluded through `.git/info/exclude`; it does not
change tracked project files.

Inspect with normal Git and factual girelay status:

```bash
git -C .girelay/workspaces/login-fix status
git -C .girelay/workspaces/login-fix diff
girelay status login-fix
```

To continue with another agent:

```bash
girelay setup claude
girelay relay login-fix -- claude
```

Review the worktree, then merge from the source checkout:

```bash
girelay merge login-fix --dry-run
girelay merge login-fix --strategy squash --message "fix: stabilize login timeout"
```

The preview is read-only. It shows the exact source/task commits, proposed
message, changed paths, configured checks, parallel-task overlap, source
divergence, confirmed committed-state conflicts, warnings, and conceptual
rollback refs. Checks remain `pending` until the real merge runs them.

Use `--strategy preserve` when the task's commits are already meaningful and
should remain in source history.

Finally remove the worktree while retaining `agent/login-fix`:

```bash
girelay clean login-fix
```

Inspect recovery points at any time:

```bash
girelay recover list login-fix
```

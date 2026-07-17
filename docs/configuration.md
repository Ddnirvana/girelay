# Configuration

girelay creates excluded local configuration at `.girelay/config.toml` on the
first `start`:

```toml
[workspace]
root = ".girelay/workspaces"
base = "main"
branch_prefix = "agent/"

[merge]
default_target = "main"
run_checks = true
check_commands = []
```

Relative workspace roots are resolved from the source checkout. Keep them
under `.girelay/` unless you have a specific reason to place linked worktrees
elsewhere.

Add project checks that must pass before a task branch is finalized or source
history changes:

```toml
[merge]
check_commands = [
  "cargo test",
  "cargo clippy --all-targets -- -D warnings",
]
```

Checks execute in the task worktree through `sh -c`. `merge --no-checks` is an
explicit bypass and should be reserved for cases where the configured command
cannot run in the current environment.

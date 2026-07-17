# Parallel And Relay Flow

This example starts two independent task worktrees, relays one task to a second
shell agent, merges both tasks, and cleans their worktrees.

```bash
PATH="$PWD/target/debug:$PATH" bash examples/multi-agent-flow/run.sh
```

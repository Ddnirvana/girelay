# Basic Agent Flow

This deterministic example creates a disposable repository and exercises:

```text
start -> status -> merge --strategy squash -> clean
```

Run from the girelay repository root after `cargo build`:

```bash
PATH="$PWD/target/debug:$PATH" bash examples/basic-agent-flow/run.sh
```

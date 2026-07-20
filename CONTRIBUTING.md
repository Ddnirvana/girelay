# Contributing to girelay

## Local Setup

```bash
cargo build
bash scripts/validate.sh
```

The crate supports Rust 1.85 and newer. CI checks the minimum version
separately and runs the full validation gate on the latest stable toolchain.
Before release-facing changes, update stable and reproduce that gate locally:

```bash
rustup update stable
rustup run stable bash scripts/validate.sh
```

The Rust CLI lives in `crates/girelay`. Integration tests use disposable Git
repositories and must never mutate the real checkout beyond normal build output.

## Tests Required

Add or update tests when changing:

- worktree creation, source discovery, or ownership checks;
- task locks, process exit propagation, or stale-session recovery;
- hidden snapshots and temporary-index cleanup;
- semantic report validation or trust labels;
- squash/preserve merge and rollback ordering;
- cleanup, branch deletion, archives, or recovery;
- JSON fields, schemas, CLI help, examples, or user documentation.

## Safety Review

Changes to Git history or filesystem cleanup must state:

- preconditions and owned resources;
- mutation order;
- rollback or compensation behavior;
- concurrent interference considered;
- tests proving refusal and failure paths.

Do not add remote mutation, force push, tracked girelay metadata, inferred agent
reasoning, or silent recovery-point deletion.

## Pull Requests

Include what changed, why, validation commands, user-visible behavior, and
safety implications.

Maintainers preparing a tag should also follow [the release procedure](docs/releasing.md).

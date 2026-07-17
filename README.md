# girelay

[![CI](https://github.com/Ddnirvana/girelay/actions/workflows/ci.yml/badge.svg)](https://github.com/Ddnirvana/girelay/actions/workflows/ci.yml)
![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-2024-orange.svg)

**Git worktrees built for coding-agent relay.**

girelay gives every agent task an isolated native Git worktree, lets another
agent continue the same durable task, merges reviewed work back to the source
branch, and keeps recovery points before anything destructive happens.

```bash
girelay setup codex
girelay start auth-fix --intent "Fix token refresh races and run auth tests" -- codex
girelay relay auth-fix -- claude
girelay merge auth-fix --strategy squash
girelay clean auth-fix
```

Git remains the storage and collaboration layer. girelay does not push, open
pull requests, host repositories, or pretend it can infer an agent's reasoning.

![Deterministic girelay lifecycle showing start, semantic relay, squash merge, rollback ref, and branch-retaining cleanup](assets/demo/girelay-run-demo.gif)

The animation is generated from a real deterministic CLI session. See the
[transcript](assets/demo/girelay-run-demo-transcript.txt) and
[MP4](assets/demo/girelay-run-demo.mp4).

## Why It Exists

`git worktree` solves checkout isolation. It does not define a task, prevent two
agents from entering the same workspace, preserve uncommitted state at relay
boundaries, or tell the next agent what was decided and what remains.

girelay adds those session semantics:

- one `agent/<task>` branch and native worktree per task;
- source-owned metadata excluded through `.git/info/exclude`;
- exclusive task locks while an agent, merge, or cleanup is active;
- hidden Git snapshots that preserve committed and uncommitted state without
  adding checkpoint commits to the task branch;
- optional semantic reports written by an installed agent skill;
- source-side squash or history-preserving merge with rollback refs;
- worktree cleanup that retains the task branch by default;
- verified bundle archives and guarded recovery.

## Install

```bash
cargo install --path crates/girelay
girelay --version
```

Registry publishing is intentionally disabled until the first coordinated
release. See [installation](docs/installation.md) for release binaries,
Homebrew, and Debian packaging.

## First Task

From a clean source checkout on `main`:

```bash
girelay setup codex
girelay start parser-fix \
  --intent "Recover malformed bodies without changing valid parsing" \
  -- codex
```

The worktree is `.girelay/workspaces/parser-fix`. Inspect it with normal Git:

```bash
git -C .girelay/workspaces/parser-fix status
git -C .girelay/workspaces/parser-fix diff
girelay status parser-fix
```

Continue with another agent when useful:

```bash
girelay setup claude
girelay relay parser-fix -- claude
```

Then merge from the clean source checkout:

```bash
girelay merge parser-fix --strategy squash --message "fix: recover malformed bodies"
girelay clean parser-fix
```

`clean` removes only the worktree. `agent/parser-fix`, snapshots, and rollback
refs remain. Branch deletion is a separate guarded choice:

```bash
girelay clean parser-fix --delete-branch
```

## What Relay Means

girelay separates facts by trust source:

- **observed by girelay:** Git snapshots, changed paths, process exit status,
  timestamps, and lock state;
- **reported by agent:** summary, decisions, failed attempts, tests, risks,
  blockers, remaining work, and next action.

The second category is available only when the agent follows the girelay skill
protocol. Without a skill, isolation, parallel worktrees, merge, cleanup, and
recovery still work; semantic handoff is honestly shown as `not reported`.

Read [Agent Relay: From Worktrees to Durable Handoffs](docs/agent-workflow.md)
for the complete model and examples.

## Commands

| Command | Purpose |
| --- | --- |
| `setup <codex|claude>` | Install the semantic relay skill at user scope. |
| `start <task> --intent ... -- <agent>` | Create a worktree and optionally run the first agent. |
| `relay <task> -- <agent>` | Continue the same task in another recorded session. |
| `status [task] [--json]` | Report factual workspace, session, report, and merge state. |
| `merge <task> --strategy squash|preserve` | Check and integrate work into the source branch. |
| `clean <task>` | Remove the worktree while retaining recoverable Git state. |
| `recover list|show|restore` | Inspect or restore snapshots, rollback refs, and archives. |

See [command reference](docs/commands.md) for flags and refusal behavior.
The [documentation index](docs/README.md) links the complete user, safety,
integration, and maintainer references.

## Agent Compatibility

| Agent | Current evidence |
| --- | --- |
| Codex CLI | Authenticated v2 lifecycle: isolated repair, semantic report, external test verification, squash merge, rollback refs, and cleanup. |
| Claude Code | User-level skill and deterministic protocol artifacts validated; live CLI not available in the current evidence environment. |
| Generic shell agents | Deterministic start, relay, merge, and clean lifecycle in every validation run. |

See the [reviewed Codex evidence](docs/evidence/codex-v2-live-2026-07-15.md)
and [evidence-level definitions](docs/agent-compatibility.md).

## Parallel Coding

Start independent tasks from the same source checkout:

```bash
girelay start auth-fix --intent "Fix auth timeout" -- codex
girelay start docs-sync --intent "Update auth documentation" -- claude
girelay status
```

Each agent receives a separate files-and-index view. This is Git isolation, not
a security sandbox: processes still share the machine, network, ports, caches,
credentials, remotes, and repository refs.

## Safety Boundary

- no network fetch, push, force push, pull request, or hosted-provider mutation;
- source checkout must be clean and on the recorded target branch before merge;
- checks run before branch finalization and source integration;
- task and source rollback refs are created before history changes;
- merge conflicts restore the clean source commit;
- dirty cleanup is refused unless archived or explicitly discarded;
- branch deletion requires an unchanged merge record and matching tips;
- stale source rollback is refused;
- `.girelay/` is excluded locally and never added to tracked project files.

The detailed contract is in [Safety Model](docs/safety.md).

## Development

```bash
cargo test
bash scripts/validate.sh
bash scripts/release-check.sh
```

## License

MIT. See [LICENSE](LICENSE).

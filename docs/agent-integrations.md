# Agent Integrations

Any command-line coding agent can use girelay's environment layer:

```bash
girelay start <task> --intent "<task>" -- <agent> [args...]
girelay relay <task> -- <agent> [args...]
```

Semantic relay additionally requires the agent to read girelay environment
variables and submit a report before it exits.

## Codex

```bash
girelay setup codex
girelay start parser-fix --intent "Fix parser recovery" -- codex
```

The setup command installs a user-level `girelay` skill under
`~/.codex/skills/girelay/SKILL.md`. It does not alter the project's tracked
`AGENTS.md`.

For non-interactive Codex CLI usage, pass the normal Codex arguments after the
separator:

```bash
girelay start parser-fix --intent "Fix parser recovery" -- \
  codex exec "Implement the intent, run focused tests, and follow the girelay skill"
```

## Claude Code

```bash
girelay setup claude
girelay relay parser-fix -- claude
```

The skill is installed under `~/.claude/skills/girelay/SKILL.md`. girelay does
not modify tracked `CLAUDE.md` or install hooks.

## Generic Agents

Without a native skill, the agent still receives `GIRELAY_*` environment
variables. An adapter can generate the documented report JSON and invoke the
hidden command in `GIRELAY_REPORT_COMMAND`. If it does not, status honestly
shows no semantic report.

## Evidence Levels

Run deterministic compatibility checks:

```bash
bash scripts/agent-matrix.sh
```

This executes a real generic shell lifecycle and validates Codex/Claude skill
artifacts. It detects installed CLIs but does not invoke authenticated models.
Model-backed runs are opt-in and must be reviewed before claims are published.

Maintainers can run selected authenticated agents explicitly:

```bash
GIRELAY_LIVE_AGENTS=codex bash scripts/agent-live-matrix.sh
GIRELAY_LIVE_AGENTS=claude bash scripts/agent-live-matrix.sh
```

The live harness uses a disposable repository, verifies the resulting files
and semantic report, merges the task, and cleans the worktree. It never runs
authenticated agents as part of ordinary CI.

See [Agent Compatibility](agent-compatibility.md).

# Agent Integrations

Any command-line coding agent can use girelay's environment layer:

```bash
girelay start <task> [--intent "<durable intent>"] -- <agent> [args...]
girelay relay <task> -- <agent> [args...]
```

Semantic relay additionally requires the agent to read girelay environment
variables and submit a report before it exits.

## Codex

```bash
girelay setup codex
girelay start parser-fix -- codex
```

The setup command installs a user-level `girelay` skill under
`~/.codex/skills/girelay/SKILL.md`. It does not alter the project's tracked
`AGENTS.md`.

For non-interactive Codex CLI usage, pass the normal Codex arguments after the
separator:

```bash
girelay start parser-fix --intent "Fix parser recovery without API changes" -- \
  codex exec "Implement the intent, run focused tests, and follow the girelay skill"
```

## Claude Code

```bash
girelay setup claude
girelay relay parser-fix -- claude
```

The skill is installed under `~/.claude/skills/girelay/SKILL.md`. girelay does
not modify tracked `CLAUDE.md` or install hooks.

## Pi

```bash
girelay setup pi
girelay start parser-fix -- pi
```

Pi implements the Agent Skills standard and discovers user skills under
`~/.pi/agent/skills/`. `girelay setup pi` installs
`~/.pi/agent/skills/girelay/SKILL.md`; it does not install executable code,
modify Pi settings, or add project-local extensions.

When `PI_CODING_AGENT_DIR` is set, setup installs under that directory's
`skills/girelay/` path instead of the default user directory.

Pi also supports TypeScript extensions for custom tools, event interception,
commands, and UI. girelay does not need one: the existing session environment
and report command already provide the complete integration boundary, while a
skill supplies the semantic workflow the model must follow. Avoiding an
extension keeps Git mutation and report validation inside the girelay binary.

For non-interactive use, explicitly ask Pi to load and follow the skill:

```bash
girelay start parser-fix --intent "Fix parser recovery and run focused tests" -- \
  pi --print \
  "Read and follow the girelay skill. Implement the intent and submit its final report."
```

Custom OpenAI-compatible endpoints belong in Pi's user-level `models.json` and
should reference API keys through environment variables. girelay never writes
or manages model credentials.

## Generic Agents

Without a native skill, the agent still receives `GIRELAY_*` environment
variables. An adapter can generate the documented report JSON and invoke the
hidden command in `GIRELAY_REPORT_COMMAND`. If it does not, status honestly
shows no semantic report.

See [Generic Agent Adapter Contract](adapter-contract.md) for the stable seven
variables, absence semantics, report schema, validation rules, and trust
boundary.

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
GIRELAY_LIVE_AGENTS=pi \
  GIRELAY_PI_API_KEY="..." \
  GIRELAY_PI_BASE_URL="http://localhost:3000/v1" \
  GIRELAY_PI_MODEL="gpt-5.6-luna" \
  bash scripts/agent-live-matrix.sh
```

The live harness uses a disposable repository, verifies the resulting files
and semantic report, merges the task, and cleans the worktree. It never runs
authenticated agents as part of ordinary CI.

Set `GIRELAY_CLAUDE_BIN` when Claude Code is installed outside `PATH`. Custom
Anthropic-compatible endpoints use Claude Code's normal environment variables;
girelay does not read or persist them.

See [Agent Compatibility](agent-compatibility.md).

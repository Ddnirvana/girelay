# Defining Multi-Agent Relay Demo

This two-stage fixture demonstrates semantic relay between real coding agents.

1. Codex implements normalization and empty-result validation, runs only the
   normalization tests, and reports reserved Git names as remaining work.
2. Claude Code reads that report, verifies the completed behavior, implements reserved
   name rejection, runs the full suite, and submits its own report.
3. A human-equivalent script reviews status and merge preview, merges, verifies
   rollback refs, cleans the worktree, and restores the source rollback point.

Run the authenticated recorder with the required agent credentials and Pi
provider environment:

```bash
GIRELAY_CLAUDE_API_KEY="..." \
GIRELAY_CLAUDE_BASE_URL="http://localhost:3000" \
GIRELAY_CLAUDE_BIN="claude" \
GIRELAY_DEMO_MODEL="gpt-5.6-luna" \
bash scripts/record-multi-agent-demo.sh
```

The script uses a disposable repository. Authenticated runs are never part of
ordinary CI. Review generated artifacts for paths and credentials before moving
them from `target/multi-agent-demo/` into `assets/demo/`. Set
`GIRELAY_DEMO_SECOND_AGENT=pi` to record the optional Codex-to-Pi variant.

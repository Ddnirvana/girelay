# Authenticated Multi-Agent Relay - 2026-07-20

## Claim

This is the defining girelay scenario: one real coding agent stops at a useful
boundary, another real agent continues the same durable task, and girelay keeps
the Git state, semantic handoff, merge review, cleanup, and rollback coherent.

The reviewed run used Codex CLI 0.144.3 for stage one and official Claude Code
2.1.215 with GPT-5.6 Luna for stage two through an Anthropic-compatible local
endpoint. It ran entirely in a disposable repository.

## Task

The public fixture is
[`examples/defining-relay-demo`](../../examples/defining-relay-demo/README.md).
Its task-id normalizer had two independent stages:

1. normalize case and separators and reject an empty result;
2. reject reserved Git names after normalization.

Codex was asked to implement only stage one, run its focused tests, and report
stage two as remaining. Claude Code was then asked to read and verify that report before
finishing stage two and running the complete suite.

## Reviewed Evidence

- Both agents ran in the same `agent/task-id-safety` native worktree, in
  separate serialized sessions with separate start/end snapshots.
- Codex's report explicitly recorded focused tests, remaining reserved-name
  behavior, the expected full-suite risk, and the next action.
- Claude Code's report explicitly recorded verification of Codex's report against the
  current diff and tests before completing the remaining behavior.
- The harness independently verified that only `task_id.py` changed and all
  three tests passed.
- Human-readable status labeled Pi's summary `reported-by-agent`.
- Merge preview was non-mutating and planned both rollback refs.
- Squash merge changed only `task_id.py`; Git verified both rollback refs.
- Cleanup removed the worktree and retained `agent/task-id-safety`.
- Guarded source recovery restored `main` exactly to its pre-merge baseline.

The reviewed [transcript](../../assets/demo/multi-agent-relay-transcript.txt),
[test output](../../assets/demo/multi-agent-relay-tests.txt), GIF, and MP4 are
generated from this run. The recorder is
[`scripts/record-multi-agent-demo.sh`](../../scripts/record-multi-agent-demo.sh).

## Trust And Privacy

Semantic summaries, decisions, failed approaches, risks, and next actions are
agent claims. The harness separately verified files, Git state, test output,
session reports, merge refs, cleanup, and rollback. Published artifacts contain
no raw model logs, API keys, user paths, disposable paths, or unrelated local
content.

## Boundary

The run proves Claude Code runtime integration through the generic adapter and
skill protocol. Its model was GPT-5.6 Luna behind an Anthropic-compatible local
endpoint, not an Anthropic-hosted Claude model. Pi has separate authenticated
evidence; the workflow and generic adapter contract remain agent-independent.

Machine-readable summary:
[multi-agent-relay-2026-07-20.json](multi-agent-relay-2026-07-20.json).

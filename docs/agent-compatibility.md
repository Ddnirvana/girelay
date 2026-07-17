# Agent Compatibility

Compatibility claims use explicit evidence levels.

| Level | Meaning |
| --- | --- |
| `verified-live` | A real process completed the defined v2 lifecycle. |
| `authenticated-live` | A real authenticated model completed and reported a reviewed task. |
| `artifacts-validated` | Skill/schema/install artifacts passed deterministic checks. |
| `cli-present-not-invoked` | A binary was detected; no agent task ran. |
| `unavailable` | The binary was not present in the test environment. |
| `failed` | The selected evidence-producing scenario failed. |

## Current Deterministic Matrix

| Runtime | Evidence |
| --- | --- |
| Generic shell command | `verified-live` through start, relay, merge, and clean. |
| Codex CLI 0.144.3 | `authenticated-live`; see [reviewed v2 evidence](evidence/codex-v2-live-2026-07-15.md). |
| Claude Code | Skill artifact validated; CLI presence reported separately. |

Run:

```bash
PATH="$PWD/target/debug:$PATH" bash scripts/agent-matrix.sh
```

Reports are written under `target/agent-matrix/` and are not published
automatically. Binary detection is not proof of semantic-report compliance or
model quality.

## Live Validation Contract

A publishable authenticated result must prove:

- agent process ran inside the recorded native worktree;
- source checkout remained clean during the session;
- task intent and session environment were available;
- only expected project files changed;
- focused tests failed before or were otherwise meaningful;
- tests passed after the agent's work;
- a schema-valid semantic report was submitted;
- report claims match observable files and command evidence;
- source merge created rollback refs and the expected result;
- cleanup retained or deleted the branch according to the chosen policy.

Raw model logs, credentials, private paths, and unrelated repository content
must never be committed as compatibility evidence.

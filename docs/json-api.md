# JSON Contracts

girelay uses schema version `2` for the focused worktree/relay lifecycle. JSON
files under `schemas/` are the public machine-readable contracts.

## Status

```bash
girelay status [<task>] --json
```

```json
{
  "schema_version": 2,
  "repo": "/path/to/source",
  "tasks": [
    {
      "id": "parser-fix",
      "intent": "Recover malformed bodies",
      "intent_source": "explicit",
      "source_branch": "main",
      "branch": "agent/parser-fix",
      "workspace": "/path/to/source/.girelay/workspaces/parser-fix",
      "lifecycle": "active",
      "activity": "idle",
      "workspace_present": true,
      "state": "paused",
      "dirty": true,
      "changed_files": ["src/parser.rs"],
      "active_session_id": null,
      "active_session": null,
      "latest_session_id": "...",
      "latest_session": {
        "id": "...",
        "agent": "codex",
        "state": "completed",
        "exit_code": 0,
        "started_at": "...",
        "finished_at": "..."
      },
      "report_available": true,
      "latest_report": {
        "available": true,
        "trust": "reported-by-agent",
        "summary": "Parser recovery implemented"
      },
      "merged_commit": null,
      "recovery_points": 2,
      "divergence": {
        "source_state": "advanced",
        "base_commit": "...",
        "source_tip": "...",
        "source_ahead_of_base": 1,
        "source_behind_base": 0,
        "task_tip": "...",
        "task_relation_to_source": "diverged",
        "task_ahead_of_source": 1,
        "task_behind_source": 1
      },
      "overlaps": [{"task_id": "docs-sync", "paths": ["src/parser.rs"]}],
      "warnings": [],
      "blockers": [],
      "next_action": "girelay merge parser-fix --dry-run"
    }
  ]
}
```

Existing status fields retain their meanings. Richer fields are additive.
Overlap is a path-set warning, not proof of a conflict. Semantic summaries are
always labeled `reported-by-agent` and may be absent.

## Merge Preview

```bash
girelay merge <task> --strategy squash --dry-run --json
```

The `merge-plan.schema.json` result contains source/task commits, proposed
message and its deterministic source, dirty finalization requirement, changed
paths, task commits, checks (`pending` or `skipped`), graph divergence,
active-task overlaps, confirmed committed-state conflicts, structured warnings,
and conceptual rollback refs ending in `<merge-id>`.

Dry-run creates no files, refs, commits, locks, merge records, worktree/index
changes, or check side effects. Preview and real merge share the same planner.

## Merge Result

```bash
girelay merge <task> --strategy squash --json
```

The result includes exact source before/after commits, finalized task tip,
changed files, executed checks, structured warnings, and both rollback refs.
`--json` alone is a real, mutating merge; add `--dry-run` for a preview.

## Cleanup Plan

```bash
girelay clean <task> --dry-run --json
```

The plan reports workspace existence, dirty state, branch action, current tip,
preserving refs/archive, and blockers. `clean --json` requires `--dry-run`, so
cleanup JSON is always a non-mutating plan.

## Recovery

```bash
girelay recover list [<task>] --json
girelay recover show <recovery-id> --json
```

Recovery points include id, task, type, Git object when applicable,
restorability, creation timestamp, approximate size, and safety note. The list
also includes `count`, `oldest_created_at`, and `disk_usage_bytes`. The byte
total sums listed ref object sizes and archive directories; shared objects and
filesystem allocation make it approximate.

## Stale Lock Inspection

```bash
girelay recover unlock <task> --json
girelay recover unlock <task> --confirm --json
```

The `lock.schema.json` object contains operation, parent/child PIDs and
liveness, creation time, active session id, `recoverable`, and `unlocked`.
Inspection does not mutate. Confirmation refuses a live process and reports
`unlocked: true` only after transactional stale-lock recovery succeeds.

## Local Metadata And Schemas

The versioned schemas cover source-owned records and command output:

- `schemas/task.schema.json`
- `schemas/session.schema.json`
- `schemas/report.schema.json`
- `schemas/archive-manifest.schema.json`
- `schemas/cleanup-plan.schema.json`
- `schemas/merge-plan.schema.json`
- `schemas/merge.schema.json`
- `schemas/status.schema.json`
- `schemas/recovery.schema.json`
- `schemas/lock.schema.json`

Paths and timestamps are local facts. Semantic report fields always carry the
`reported-by-agent` trust label. Consumers must not relabel them as observed or
verified facts.

Patch releases preserve required fields and meanings. A breaking contract
increments `schema_version` and the schema filenames together.

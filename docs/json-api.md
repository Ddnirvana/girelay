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
      "branch": "agent/parser-fix",
      "workspace": "/path/to/source/.girelay/workspaces/parser-fix",
      "state": "paused",
      "dirty": true,
      "changed_files": ["src/parser.rs"],
      "active_session_id": null,
      "latest_session_id": "...",
      "report_available": true,
      "merged_commit": null,
      "blockers": []
    }
  ]
}
```

## Merge

```bash
girelay merge <task> --strategy squash --json
```

The result includes exact source before/after commits, finalized task tip,
changed files, checks, and both rollback refs. Agent child output is not wrapped
as JSON; use the session records for process facts.

## Cleanup Plan

```bash
girelay clean <task> --dry-run --json
```

The plan reports workspace existence, dirty state, branch action, current tip,
preserving refs/archive, and blockers. `--json` implies no mutation.

## Recovery

```bash
girelay recover list [<task>] --json
girelay recover show <recovery-id> --json
```

Recovery points include id, task, type, Git object when applicable,
restorability, and safety note.

## Local Metadata

The same versioned schemas cover source-owned records:

- `schemas/task.schema.json`
- `schemas/session.schema.json`
- `schemas/report.schema.json`
- `schemas/archive-manifest.schema.json`

Paths and timestamps are local facts. Semantic report fields always carry the
`reported-by-agent` trust label. Consumers must not relabel them as observed or
verified facts.

Patch releases preserve required fields and meanings. A breaking contract
increments `schema_version` and the schema filenames together.

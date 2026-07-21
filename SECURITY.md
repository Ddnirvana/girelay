# Security Policy

girelay is a local developer tool that creates native Git worktrees, launches
agent processes, merges local branches, removes owned worktrees, and restores
local recovery points. It never mutates remotes.

Report security or data-safety issues through [GitHub private vulnerability
reporting](https://github.com/Ddnirvana/girelay/security/advisories/new). Do not
open a public issue for an undisclosed vulnerability.

Safety-sensitive issues include:

- deleting dirty or unrelated worktrees without explicit authorization;
- changing source history without rollback refs or revalidation;
- accepting stale source rollback after the branch advanced;
- allowing two sessions to write one task concurrently;
- committing `.girelay/` metadata accidentally;
- leaking secrets through session or semantic-report metadata;
- restoring a corrupt or mismatched archive;
- treating agent-reported claims as observed facts;
- introducing push or force-push behavior.

## Supported Versions

girelay is pre-1.0. Security fixes target the latest `0.1.x` release and the
latest `main` branch. Upgrade to the newest patch before reporting an issue
that may already be fixed.

| Version | Supported |
| --- | --- |
| 0.1.x | Yes |
| Earlier prototypes | No |

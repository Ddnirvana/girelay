# Threat Model

girelay assumes Git and the selected agent executable are trusted enough to run
with the user's permissions. Repository content and agent output may be
untrusted.

## Protected Assets

- source checkout files and branch history;
- task worktree committed and uncommitted state;
- excluded task/session/report metadata;
- an inspectable recovery path before merge and cleanup;
- command-line secrets in common named argument forms.

## Defenses

- task ids reject separators, whitespace, leading/trailing dots, and shell
  metacharacters;
- child processes use argument arrays rather than interpolated shell commands;
- common token/password flags are redacted from session records;
- metadata writes use same-directory temporary files and atomic rename;
- native worktree source/branch ownership is verified;
- source-owned locks serialize operations on one task;
- snapshots use isolated temporary indexes;
- source cleanliness and head are revalidated immediately before merge;
- task and source rollback refs precede history mutation;
- cleanup uses `git worktree remove` and explicit dirty-state choices;
- branch deletion and source recovery require exact recorded state;
- archive checksum and Git bundle are verified before restore;
- no remote mutation or force-push code exists.

## Residual Risks

- Agents can access anything allowed by their operating-system sandbox.
- Worktrees share refs, object storage, hooks, remotes, and repository config.
- Configured merge checks execute repository-controlled shell commands.
- Redaction cannot identify every positional or custom secret format.
- A killed parent can leave a stale lock and incomplete session record.
- Reports are agent claims and can be mistaken or dishonest.
- Concurrent tasks may edit overlapping files and conflict at merge time.

Use an operating-system sandbox for untrusted agents, inspect repository hooks
and configured checks, and keep credentials out of intents and report text.

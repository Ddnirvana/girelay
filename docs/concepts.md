# Concepts

## Source Checkout

The main checkout where tasks are created and merged. girelay requires it to be
clean before `start` and `merge`.

## Task

A durable id and intent bound to one `agent/<task>` branch and one native linked
worktree under `.girelay/workspaces/<task>`. Intent defaults verbatim to the
task id and can be made more precise with optional `--intent`.

## Session

One agent process launched by `start` or `relay`. Sessions record sanitized
command arguments, timestamps, exit status, changed paths, and hidden start/end
snapshots. A per-task lock prevents overlapping sessions.

## Snapshot

A hidden commit under `refs/girelay/snapshots/<task>/<session>/<phase>`. girelay
builds it with a temporary Git index, so staged, unstaged, and untracked task
content is preserved without changing the worktree's index or branch history.

## Semantic Report

A schema-v2 report submitted by an agent during its active session. It carries
summary, completed and remaining work, decisions, failed attempts, blockers,
tests, risks, and next action. These are labeled `reported-by-agent`; girelay
does not infer them from a diff.

## Relay

Starting a new session for an existing task. The next agent receives the same
worktree, durable intent, and the prior semantic report path when available.
The installed skill tells the agent to verify reported claims against files and
Git state before continuing.

## Merge Record

The immutable facts written after source integration: strategy, source before
and after commits, task tip, target branch, and task/source rollback refs.
Cleanup uses this record to decide whether branch deletion is still safe.

## Recovery Point

A relay snapshot, task rollback, source pre-merge rollback, or verified cleanup
archive. Snapshot and task rollback recovery create a new branch/worktree;
source rollback is allowed only while the source still exactly matches the
recorded merge result.

## Overlap And Divergence

girelay compares committed and dirty changed paths across active tasks. Shared
paths are an overlap warning for human coordination, not proof of a Git
conflict. Separately, graph divergence records whether the source is unchanged,
advanced, behind, or diverged from task creation and how the task tip relates
to the current source. girelay reports these facts but never automatically
rebases, resets, or rewrites a task.

## Merge Plan

`merge --dry-run` computes the same deterministic plan consumed by a real
merge: source/task commits, message, paths, commits, dirty finalization, checks,
divergence, overlap, committed-state conflict preflight, warnings, and planned
rollback refs. It neither runs checks nor mutates files, refs, locks, commits,
or metadata.

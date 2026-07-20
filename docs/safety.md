# Safety Model

girelay manages Git state conservatively, but it is not a process sandbox.

## Ownership

- Tasks use native linked worktrees owned by the source repository.
- Task metadata lives only under the source checkout's `.girelay/` directory.
- `.girelay/` is added to the common Git directory's `info/exclude` file.
- Before session or cleanup operations, girelay verifies the recorded source,
  worktree, and branch relationship.
- One source-owned lock serializes agent sessions, merge, and cleanup per task.

## Snapshot Isolation

Every agent session gets hidden start and end commits under:

```text
refs/girelay/snapshots/<task>/<session>/<phase>
```

Snapshots use a temporary index. They include tracked modifications, staged
changes, deletions, and untracked files, while leaving the task branch and its
real index unchanged.

Hidden commits are recovery evidence, not user-visible checkpoint history.

## Concurrent Task Awareness

Status and merge planning compare committed and working-tree paths across active
tasks. This includes staged, unstaged, renamed, deleted, and untracked paths.
Overlap is reported only as a warning: matching paths do not prove a textual
conflict and never cause an automatic refusal. A separate temporary-index
preflight reports conflicts confirmed by Git for committed task state. Dirty
state remains a separate warning.

## Merge Transaction

`merge` requires:

- invocation from the source checkout;
- source on the task's recorded base branch;
- clean source files and index;
- existing task worktree on its recorded branch;
- no active task lock;
- passing configured checks unless explicitly bypassed.

The order is deliberate:

```text
check task -> snapshot -> task rollback -> finalize task
           -> revalidate source -> source rollback -> integrate
```

On merge failure, girelay aborts merge state when present and hard-resets only
the previously verified-clean source checkout to its recorded pre-merge commit.
The task branch, worktree, snapshots, and rollback refs remain.

`merge --dry-run` uses the same planning code without acquiring a lock, running
checks, or creating files, refs, commits, merge records, or index changes.
Configured checks are marked `pending` or `skipped`; preview never claims they
passed. Source advancement and divergence are reported, but girelay never
automatically rebases, resets, or rewrites task history.

## Cleanup

`clean` removes a linked worktree through `git worktree remove`, never recursive
filesystem deletion of an arbitrary recorded path.

Default behavior retains `agent/<task>`. Dirty work blocks cleanup unless:

- `--archive` captures a pre-clean snapshot and verified self-contained bundle;
  or
- `--discard-uncommitted` explicitly accepts loss of uncommitted files.

Archive publication and verification finish before worktree removal. Each
manifest records the original branch tip, recoverable snapshot commit/ref,
bundle SHA-256, task identity, and metadata paths.

Branch deletion is not ordinary cleanup. It is allowed only when:

- the task has a merge record;
- source is clean and on the recorded target;
- source `HEAD` still equals the recorded merge result;
- task branch still equals the recorded task tip;
- task and source rollback refs still exist.

This works for squash merges without relying on ancestry.

## Recovery

- Relay snapshots and task rollback refs restore to a new branch/worktree.
- Source rollback refuses dirty source, wrong branch, changed source head, or a
  ref that does not match the task's current merge record.
- Archive restore verifies checksum and bundle before changing refs.
- Existing divergent task branches are never overwritten.
- Recovery never mutates remotes or force pushes.
- Stale locks are inspected and recovered only through `recover unlock`.
- Unlock refuses when a recorded parent or child process is alive.
- Interrupted sessions are snapshotted and closed before a stale lock is
  retired; non-session operation locks are retired without inventing a session.

## Trust Labels

girelay observes Git and process facts. It cannot observe an agent's internal
reasoning. Semantic report fields remain labeled `reported-by-agent` and should
be verified by the next agent or human.

## Explicit Non-Goals

girelay does not isolate processes, credentials, network, services, ports,
caches, submodules, Git refs, or remotes. It never performs a network fetch,
pushes, force pushes, creates pull requests, or mutates a hosting provider.
Archive restore may fetch objects from its verified local Git bundle.

# Agent Relay: From Worktrees to Durable Handoffs

Coding agents changed the unit of development. A human usually opens one
checkout, understands the current branch, makes a bounded set of edits, and
decides when to commit. An agent may run for an hour, create many intermediate
states, stop at a tool failure, or hand unfinished work to another model.

Git still stores the code correctly. The missing piece is a small local layer
that gives agent work an explicit lifecycle.

## 1. The Normal Git Workflow

A developer commonly works like this:

```bash
git switch -c fix/login-timeout
# edit, test, commit
git switch main
git merge fix/login-timeout
```

This works because the human carries context in their head:

- why the branch exists;
- what has already been tried;
- which failures are expected;
- what remains;
- whether the current diff is ready to merge.

Git records files, trees, commits, and refs. It deliberately does not record
that working memory.

## 2. What Changes With Agents

When several agents share one checkout, ordinary operations collide:

```text
Agent A changes src/auth.rs
Agent B switches the branch
Agent C stages generated files
Agent A runs tests against B's files
Human cannot tell which session owns the index
```

The first improvement is isolation. Git already provides it.

## 3. Worktrees Solve File Isolation

```bash
git worktree add .worktrees/auth -b agent/auth main
git worktree add .worktrees/docs -b agent/docs main
```

Now each agent has its own working tree and index:

```text
source checkout
├── main
├── .worktrees/auth  -> agent/auth
└── .worktrees/docs  -> agent/docs
```

That is enough for many parallel tasks. It is not a complete agent workflow.
You still need to answer:

1. Which worktree belongs to which durable task?
2. Is an agent currently using it?
3. What was the exact state before and after each session?
4. Did the previous agent finish, fail, or get interrupted?
5. What did it decide, try, test, and leave unresolved?
6. How should uncommitted work be merged without noisy checkpoint commits?
7. When is cleanup safe, and how can lost state be recovered?

Shell scripts can create worktrees. The difficult part is keeping those
answers consistent when sessions fail or overlap.

## 4. Two Layers, Not One Magical Agent Manager

girelay separates the problem into two honest layers.

### Layer A: Git Environment Management

This works for every command-line agent, even if the agent knows nothing about
girelay:

- create the task branch and native worktree;
- run the process in that directory;
- serialize access with a task lock;
- record process and Git facts;
- preserve uncommitted state in hidden snapshots;
- merge, clean, and recover conservatively.

### Layer B: Semantic Relay Protocol

This requires agent cooperation through `girelay setup codex` or
`girelay setup claude`:

- read the durable task intent;
- inspect the previous report when present;
- verify its claims against current code;
- report completed and remaining work;
- record decisions and failed approaches;
- list tests actually run, blockers, risks, and next action.

girelay cannot reconstruct this semantic context from a diff. A changed parser
can show *what* changed; it cannot prove why one recovery strategy was rejected
or whether a test was genuinely executed.

## 5. The Small Lifecycle

```text
setup -> start -> relay -> merge -> clean
                    |        |
                  status   recover
```

There are seven user commands:

```text
setup  start  relay  status  merge  clean  recover
```

No extra checkpoint ceremony, staged integration-plan vocabulary,
pull-request generation, or provider automation is required.

## 6. Start One Isolated Agent

Install the protocol skill once:

```bash
girelay setup codex
```

Then start a task from a clean source checkout:

```bash
girelay start auth-fix -- codex
```

That short form stores `auth-fix` itself as the durable intent. When the task id
does not carry enough context, make intent explicit without changing the
lifecycle:

```bash
girelay start auth-fix \
  --intent "Fix token refresh races without changing public API" \
  -- codex
```

girelay creates:

```text
source/.girelay/
├── config.toml
├── tasks/auth-fix.json
├── sessions/auth-fix/<session>.json
├── reports/auth-fix/<session>.json     # only if agent reports
├── locks/
└── workspaces/auth-fix/                # Git linked worktree
```

The agent starts inside `workspaces/auth-fix`, not the source checkout. The
source remains available for review or other tasks.

## 7. What A Session Captures

Before launching the child, girelay builds a start snapshot with a temporary
index. After the child exits, it builds an end snapshot the same way.

```text
refs/girelay/snapshots/auth-fix/<session>/start
refs/girelay/snapshots/auth-fix/<session>/end
```

These commits can contain untracked and unstaged files, but they do not move
`agent/auth-fix` and do not alter the worktree index.

The session record distinguishes evidence sources:

```json
{
  "state": "completed",
  "exit_code": 0,
  "changed_files": ["src/auth.rs", "tests/auth.rs"],
  "trust": {
    "git_state": "observed-by-girelay",
    "process_result": "observed-by-girelay",
    "semantic_report": "reported-by-agent"
  }
}
```

If the agent never reports, `semantic_report` is `not-reported`. Nothing else
breaks.

## 8. Parallel Coding Is A Consequence

```bash
girelay start auth-fix -- codex
girelay start docs-sync -- claude
girelay start parser-tests -- codex
girelay status
```

Each task has its own files and index. Sessions for different tasks may run in
parallel. Sessions for the same task are serialized.

The repository dashboard also compares complete changed-path sets across active
tasks. If `auth-fix` and `docs-sync` both touch `src/auth.rs`, both rows identify
the other task and path. This is an early coordination warning, not a claim that
Git will conflict and not a reason for automatic refusal. The set includes
committed, staged, unstaged, renamed, deleted, and untracked paths.

This is not security isolation. Agents still share operating-system resources,
ports, caches, credentials, network access, remotes, and the repository's ref
database. Use containers or sandboxes when those boundaries matter.

## 9. Relay To Another Agent

Suppose Codex implemented header parsing but body recovery is unfinished:

```bash
girelay relay parser-fix -- claude
```

Claude receives:

- the same worktree and branch;
- the original `GIRELAY_INTENT`;
- a new session id and start snapshot;
- `GIRELAY_PREVIOUS_REPORT` when Codex submitted one.

The report looks like:

```json
{
  "schema_version": 2,
  "task_id": "parser-fix",
  "session_id": "...",
  "agent": "codex",
  "start_snapshot": "...",
  "end_snapshot": null,
  "summary": "Header recovery is implemented; body resynchronization remains.",
  "completed": ["parse duplicate headers", "add header regression tests"],
  "remaining": ["recover after malformed body length"],
  "decisions": ["keep valid-input fast path unchanged"],
  "failed_attempts": ["byte-by-byte scan was quadratic on the corpus"],
  "blockers": [],
  "tests": ["cargo test header_parser"],
  "risks": ["body recovery still needs fuzz coverage"],
  "next_action": "implement bounded delimiter resynchronization",
  "trust": "reported-by-agent"
}
```

The next agent must treat this as a report, not proof. The installed skill asks
it to inspect the current files, snapshots, and tests before relying on claims.

That is the relay: durable code state plus explicit, attributable semantic
context.

## 10. Merge Without Checkpoint Noise

Review with normal Git:

```bash
git -C .girelay/workspaces/parser-fix status
git -C .girelay/workspaces/parser-fix diff
git -C .girelay/workspaces/parser-fix log --oneline main..HEAD
```

Ask girelay for a non-mutating integration plan:

```bash
girelay merge parser-fix --strategy squash --dry-run
```

The plan uses exact Git graph facts to report source advancement, task relation,
active-task path overlap, commits, changed paths, dirty finalization, and any
conflict confirmed for committed state. Every deterministic warning includes
its evidence and a safe next action. Configured checks remain `pending` because
preview does not execute them. No lock, ref, commit, file, index, or metadata is
changed.

Then merge from source:

```bash
girelay merge parser-fix \
  --strategy squash \
  --message "fix(parser): recover malformed bodies"
```

If agents made no commits, girelay runs configured checks and creates one final
task commit. It then creates task and source rollback refs, revalidates source,
and produces one source commit.

When `--message` is omitted, girelay uses an explicitly supplied durable intent.
If the intent defaulted from the task id, the stable message is `agent: complete
<task>`. It never promotes agent-reported prose into verified commit input.

If the task already has meaningful commits:

```bash
girelay merge parser-fix --strategy preserve
```

This performs a normal non-fast-forward merge. Conflicts abort and restore the
previously clean source checkout.

## 11. Cleanup Is Separate From Merge

```bash
girelay clean parser-fix
```

This removes the linked worktree but retains `agent/parser-fix`. Keeping merge
and cleanup separate makes review and recovery easier.

For valuable dirty work that should be removed from disk:

```bash
girelay clean parser-fix --archive
```

For branch deletion after a recorded merge:

```bash
girelay clean parser-fix --delete-branch
```

The deletion is refused if either source or task changed after merge.

## 12. Recovery Is Inspectable

```bash
girelay recover list parser-fix
girelay recover show <recovery-id>
girelay recover restore <recovery-id> --confirm
```

Snapshot recovery creates a new branch and worktree. Source rollback works only
for the exact current merge result. Archive recovery verifies both checksum and
Git bundle before recreating a task.

If an interrupted process leaves a task lock, inspect rather than bypass it:

```bash
girelay recover unlock parser-fix
girelay recover unlock parser-fix --confirm
```

Confirmation is refused while the recorded girelay parent or agent child is
alive. For an interrupted session, recovery snapshots current state and closes
the session record before removing the stale lock. Start the next relay only
after that operation succeeds.

## 13. What Girelay Deliberately Leaves To Git

After local merge, use your existing workflow:

```bash
git push
gh pr create
```

Or use GitLab, Gerrit, email patches, or no remote at all. girelay does not own
those decisions. Its job ends when agent work is isolated, relayable,
reviewably merged, cleanable, and recoverable.

## 14. Choosing The Right Level

Use plain `git worktree` when you only need another checkout.

Use girelay without skills when you need repeatable isolation, process records,
hidden snapshots, merge safety, and cleanup/recovery.

Use girelay with agent skills when work may move between people or agents and
semantic context matters as much as the diff.

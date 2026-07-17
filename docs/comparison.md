# Comparison

| Tool | Primary job | How girelay differs |
| --- | --- | --- |
| Git | Content-addressed history, refs, remotes | girelay keeps Git and adds a local agent-task/session protocol. |
| `git worktree` | Multiple working trees sharing one repository | girelay uses it directly, then adds intent, locks, snapshots, reports, merge records, cleanup, and recovery. |
| jj | General version-control UX with Git interoperability | girelay is a narrow agent workflow layer, not a new VCS. |
| gh/glab | Hosted-provider and review automation | girelay intentionally stops before push or pull/merge requests. |
| Herdr and terminal managers | Agent process/session orchestration | girelay manages repository state; it composes with process managers. |
| Worktree shell scripts | Custom local isolation | girelay standardizes failure records, trust labels, transactional merge, guarded cleanup, JSON, and recovery. |

Use plain worktrees when checkout isolation is enough. Use girelay when the
task must survive process failure, move between agents, merge predictably, and
remain recoverable.

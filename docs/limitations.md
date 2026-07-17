# Limitations

- Worktrees isolate files and indexes, not processes, network, ports, caches,
  credentials, refs, remotes, hooks, submodules, or services.
- Two tasks may edit the same files; overlap warnings and merge-order guidance
  are not implemented yet.
- Semantic relay depends on agent skill compliance. Missing reports are shown
  honestly and do not block environment management.
- Agent-reported tests and decisions are not independently verified by girelay.
- Configured checks use `sh -c`; projects targeting Windows should choose
  commands available in their release environment.
- Dirty archive restoration recreates the final file state as uncommitted work;
  the exact staged-versus-unstaged split is not retained.
- `preserve` always requests a non-fast-forward merge.
- Abrupt process termination requires explicit stale-lock recovery after the
  user confirms the old process is gone.
- Public packages, release downloads, Homebrew installation, and download
  badges become real only after the corresponding release is published.

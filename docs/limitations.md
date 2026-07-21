# Limitations

- Worktrees isolate files and indexes, not processes, network, ports, caches,
  credentials, refs, remotes, hooks, submodules, or services.
- Two tasks may edit the same files. girelay reports overlapping paths and
  confirmed committed-state conflicts, but warnings do not serialize tasks,
  prove a textual conflict, or choose a merge order.
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
- Package registries and release archives can lag the repository during a new
  release. Verify the requested version and checksum before installation.

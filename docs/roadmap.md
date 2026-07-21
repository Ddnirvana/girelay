# Roadmap

girelay stays focused on local agent worktree lifecycle and semantic relay.

## Current Foundation

- native task worktrees under excluded local metadata;
- serialized agent sessions with hidden start/end snapshots;
- explicit agent-reported semantic handoffs and trust labels;
- factual status and stable schema-v2 JSON;
- squash/preserve source merge with checks and rollback refs;
- dirty-state-aware cleanup, guarded branch deletion, archives, and recovery;
- Codex, Claude, and Pi skill installation without tracked-file modification;
- a stable generic environment and report protocol for additional adapters;
- active-task path overlap and exact source/task divergence reporting;
- reviewed Codex-to-Claude relay evidence with reproducible demo media.

## P0: Release Evidence

- publish checksummed binaries, Debian packages, a Homebrew formula, and the
  crate after all final registry checks;
- add Windows runtime scenarios in addition to cross-platform compilation;
- make every safety refusal name the unchanged state and safest next command.

## P1: Stronger Relay

- validate reports against optional adapter-captured command/test evidence;
- expose concise report history in factual status without treating reports as
  verified facts;
- detect missing or stale agent skills and report-schema incompatibility;
- add portable adapters only when they preserve the same trust boundary.

## P1: Better Parallel Work

- record optional task dependencies and suggested merge order;
- support safe adoption of an existing native worktree without weakening
  ownership checks.

## P1: Recovery Depth

- retain staged-versus-unstaged archive state;
- verify archive restore in a repository with partial/missing local refs;
- add retention policies that never delete the final recovery point silently;
- provide machine-readable recovery precondition failures.

## Quality Gates

- no remote mutation or force-push path;
- rollback refs before source history mutation;
- `.girelay/` remains locally excluded;
- one lock protocol for session, merge, and cleanup;
- stable JSON changes are versioned and tested;
- docs claims are backed by deterministic or reviewed live evidence;
- release checks run on Linux and macOS, with release compilation for Windows.

## Not Planned

- replacing Git storage, commits, branches, or remotes;
- pull-request or hosted-provider automation;
- a hosted orchestration service;
- automatic inference of agent decisions or blockers;
- a broad terminal/session manager;
- visual dashboards or model-generated commit planning.

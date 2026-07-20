# Changelog

All notable changes to girelay are documented here.

## 0.1.0 - Unreleased

- Added native Git worktrees under excluded `.girelay/workspaces` metadata.
- Added the focused `setup`, `start`, `relay`, `status`, `merge`, `clean`, and
  `recover` public command lifecycle.
- Added source-owned schema-v2 task, session, report, merge, cleanup, archive,
  and recovery contracts.
- Added per-task session/merge/cleanup locking with explicit stale recovery.
- Added optional intent with task-id fallback and deterministic merge messages.
- Added detailed single-task status, active-task path overlap, exact source/task
  divergence, and evidence-backed deterministic merge warnings.
- Added non-mutating merge previews shared with real merge planning, including
  pending checks, commits, paths, conflicts, and conceptual rollback refs.
- Added centralized process-aware stale-lock inspection and recovery through
  `recover unlock`, including interrupted-session preservation.
- Added readable recovery age, restorability, count, oldest-point, and
  approximate-storage reporting while retaining exact refs in detailed output.
- Added hidden start/end snapshots that preserve uncommitted state without
  adding checkpoint commits to task branches.
- Added strengthened Codex, Claude, and Pi user-level relay skills plus a stable
  generic adapter environment and immutable, session-bound semantic reports
  with explicit trust labels.
- Added squash and history-preserving source merge, configured checks, task and
  source rollback refs, source revalidation, and conflict rollback.
- Added dirty-state-aware cleanup, retained branches by default, merge-aware
  branch deletion, verified SHA-256 Git bundle archives, and guarded recovery.
- Added deterministic lifecycle, parallel, report, conflict, archive, and stale
  rollback tests plus release/package validation.
- Added focused user documentation, examples, compatibility evidence levels,
  release packaging, deterministic demo tooling, and an authenticated
  Codex-to-Claude relay demo with reviewed rollback evidence.

The pre-release prototype command and integration experiments were removed
before the first public release to keep the product scope coherent.

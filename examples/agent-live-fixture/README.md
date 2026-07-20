# Authenticated Agent Fixture

This intentionally failing Python fixture is used by
`scripts/agent-live-matrix.sh`. A compatible agent must change only
`task_id.py`, make both tests pass, and submit a schema-valid girelay semantic
report from its active task session.

The fixture is small enough for a human to review, while still requiring a real
code edit, test execution, report submission, merge, rollback-ref verification,
and cleanup. Authenticated runs are opt-in and never part of ordinary CI.

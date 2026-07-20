# Demo Video Guide

Regenerate the deterministic transcript, MP4, GIF, and social preview when
Node.js, Playwright, Chromium, and ffmpeg are available:

```bash
bash scripts/render-demo-video.sh
```

The renderer runs `scripts/demo.sh`, sanitizes temporary paths, captures the
real output, and builds the media files referenced by the README. The shell
commands stand in for agents, so label this deterministic evidence rather than
an authenticated model run.

The defining authenticated relay has a separate two-step workflow:

```bash
GIRELAY_CLAUDE_API_KEY="..." bash scripts/record-multi-agent-demo.sh
```

Review `target/multi-agent-demo/` for semantic-report accuracy, changed paths,
tests, refs, credentials, and private paths. Promote only the reviewed
transcript and tests, then render them with:

```bash
bash scripts/render-multi-agent-demo.sh
```

The checked-in authenticated transcript, GIF, MP4, test output, fixture,
recorder, agent versions, and explicit trust labels form one evidence set. Raw
agent output remains ignored under `target/` and is never published.

For the launch recording, use a disposable real repository and authenticated
agents. Review the recording for credentials, private paths, unrelated files,
and report claims before publishing. Keep the final frame on:

```text
girelay status <task>
girelay merge <task> --dry-run
source merge commit
source rollback ref
retained task branch after clean
```

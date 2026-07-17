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

For the launch recording, use a disposable real repository and authenticated
agents. Review the recording for credentials, private paths, unrelated files,
and report claims before publishing. Keep the final frame on:

```text
girelay status <task>
source merge commit
source rollback ref
retained task branch after clean
```

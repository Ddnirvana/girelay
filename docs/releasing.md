# Releasing

Release from a clean, committed checkout whose version and dated changelog
entry describe the tag being created.

## Preflight

```bash
bash scripts/release-check.sh
```

The release check runs formatting, clippy, tests, examples, the deterministic
agent matrix, dogfood scenario, release build, offline install verification,
Debian metadata checks, binary help checks, and Cargo package verification.

Changes to release metadata or packaging on `main` also trigger the private
multi-platform artifact matrix without creating a GitHub release. Use
`workflow_dispatch` when a rehearsal is needed without such a change.

## Local Artifact Package

Build a local platform archive and checksum:

```bash
bash scripts/package-release.sh
```

This writes files under `dist/`.

## Tagged Release

`.github/workflows/release.yml` is configured to:

- run `scripts/release-check.sh`
- build Linux x86_64
- build Linux arm64 and Debian packages for both Linux architectures
- build macOS x86_64
- build macOS arm64
- build Windows x86_64
- upload artifacts
- generate SHA-256 checksum files
- attach artifacts to tagged GitHub releases

It uses `GITHUB_TOKEN` only. Before tagging:

```bash
bash scripts/release-check.sh
bash scripts/package-release.sh
```

Inspect the local archive and checksum, confirm CI passes on `main`, then create
an annotated tag matching the Cargo version:

```bash
git tag -a v0.1.0 -m "girelay 0.1.0"
git push origin v0.1.0
```

After the workflow completes, download each artifact, verify its checksum, and
test every native archive and both Debian packages. Run `cargo publish --dry-run`
before `cargo publish`. Publish the Homebrew formula only after filling it with
the final release URLs and checksums and testing the exact tap install command.

Authenticated agent evidence is reviewed separately with
`scripts/agent-live-matrix.sh`; it is not a release-time network dependency.

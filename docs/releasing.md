# Releasing

Release from a clean, committed checkout whose version and changelog describe
the tag being created.

## Preflight

```bash
bash scripts/release-check.sh
```

The release check runs formatting, clippy, tests, examples, the deterministic
agent matrix, dogfood scenario, release build, offline install verification,
Debian metadata checks, binary help checks, and Cargo package verification.

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

Inspect the local archive and checksum, confirm CI passes on `main`, then tag:

```bash
git tag v0.1.0
git push origin main --tags
```

After the workflow completes, download each artifact, verify its checksum, and
test at least one clean installation. Publish the crate or Homebrew formula
only after enabling the corresponding manifest/template and testing the exact
public install command.

Authenticated agent evidence is reviewed separately with
`scripts/agent-live-matrix.sh`; it is not a release-time network dependency.

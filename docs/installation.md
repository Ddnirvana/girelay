# Installation

Prerequisites: Git 2.31 or newer and a platform supported by the release
artifacts. Building from source requires Rust 1.85 or newer.

## From crates.io

```bash
cargo install girelay
girelay --version
```

Confirm that Cargo installed the requested release before using it in a
repository:

```bash
cargo install --list | grep '^girelay v'
```

## From GitHub Releases

Download the archive and adjacent `.sha256` file for your platform from the
[GitHub releases page](https://github.com/Ddnirvana/girelay/releases). Verify
the checksum before extracting it.

Artifact names:

- `girelay-linux-x86_64.tar.gz`
- `girelay-linux-aarch64.tar.gz`
- `girelay-macos-x86_64.tar.gz`
- `girelay-macos-aarch64.tar.gz`
- `girelay-windows-x86_64.zip`

Linux release archives contain static musl binaries so they remain compatible
with stable distributions whose GLIBC is older than the release runner. On
Linux:

```bash
sha256sum -c girelay-linux-x86_64.tar.gz.sha256
tar -xzf girelay-linux-x86_64.tar.gz
sudo install -m 0755 girelay /usr/local/bin/girelay
```

On macOS, use `shasum -a 256 -c` for checksum verification. Each archive also
contains the project README and MIT license.

## Debian And Ubuntu

Checksummed `amd64` and `arm64` `.deb` packages are attached to each GitHub
release. See [Debian and Ubuntu packages](debian.md).

## From A Local Checkout

```bash
git clone https://github.com/Ddnirvana/girelay.git
cd girelay
cargo install --path crates/girelay
girelay --version
```

## From Homebrew

The repository includes a formula template for release automation, but no tap
is advertised until its install command has been tested against a tagged
release.

## Development Build

```bash
cargo build
PATH="$PWD/target/debug:$PATH" girelay --help
```

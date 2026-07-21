# Debian And Ubuntu Packages

GitHub releases are configured to attach `.deb` packages for `amd64` and `arm64`. Each package installs `girelay` under `/usr/bin` and depends on Git and CA certificates.

Install a downloaded package:

```bash
sha256sum -c girelay_0.1.1_amd64.deb.sha256
sudo apt install ./girelay_0.1.1_amd64.deb
girelay --version
```

The packages contain static musl binaries and therefore do not require the
GLIBC version used by the GitHub release runner.

Build from an existing Linux release target:

```bash
export GIRELAY_DEB_MAINTAINER="Your Name <you@example.com>"
bash scripts/package-deb.sh build x86_64-unknown-linux-musl amd64
```

There is no global `.deb` namespace. A GitHub release or private APT repository can distribute `girelay` immediately. Inclusion in Debian or Ubuntu's official archives requires a separate source-package submission, policy review, and maintainer process; this repository does not claim official archive inclusion.

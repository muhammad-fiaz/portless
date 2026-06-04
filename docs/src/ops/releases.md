# Releases

We use [GitHub Releases](https://github.com/muhammad-fiaz/portless/releases)
as the canonical changelog. Each release includes:

- Auto-generated release notes from commit history.
- Pre-built binaries for all supported platforms.
- SHA-256 checksums for each binary.
- A Debian package for Debian/Ubuntu.

## Versioning

We follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html). We are
currently pre-1.0 (`0.0.x`), so breaking changes bump the minor version
(0.1, 0.2, ...). Once we reach 1.0, breaking changes will bump the major
version.

## Supported targets

| OS | Target | Binary |
|----|--------|--------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | `portless-linux-amd64.tar.gz` |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | `portless-linux-arm64.tar.gz` |
| macOS Intel | `x86_64-apple-darwin` | `portless-macos-amd64.tar.gz` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `portless-macos-arm64.tar.gz` |
| Windows x86_64 | `x86_64-pc-windows-msvc` | `portless-windows-amd64.zip` |
| Debian/Ubuntu | `amd64` | `portless_*_amd64.deb.gz` |

## Verifying a release

Each release includes a `.sha256` file for every binary. Verify before
extracting:

```sh
# Linux/macOS
sha256sum -c portless-linux-amd64.tar.gz.sha256
```

```powershell
# Windows (PowerShell)
$hash = (Get-FileHash -Algorithm SHA256 portless-windows-amd64.zip).Hash.ToLower()
$expected = (Get-Content portless-windows-amd64.zip.sha256).Split(" ")[0]
if ($hash -eq $expected) { Write-Host "Checksum OK" } else { Write-Host "Checksum mismatch!" }
```

## Release process

The release process is fully automated via
[`.github/workflows/release.yml`](https://github.com/muhammad-fiaz/portless/blob/main/.github/workflows/release.yml):

1. Bump version in `Cargo.toml`.
2. Commit: `git commit -m "release: v0.X.Y"`.
3. Tag: `git tag -s v0.X.Y -m "v0.X.Y"`.
4. Push: `git push origin main --tags`.
5. CI builds for all targets, uploads artifacts, creates a GitHub release.

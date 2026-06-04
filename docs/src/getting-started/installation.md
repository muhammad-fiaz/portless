# Installation

## Requirements

- **Rust 1.85 or later** (only required to build from source).
- **macOS**, **Linux**, or **Windows**.

## From source

Install directly from GitHub:

```sh
cargo install --git https://github.com/muhammad-fiaz/portless
```

The binary lands in `~/.cargo/bin/portless` (or `%USERPROFILE%\.cargo\bin\portless.exe` on Windows).

To install from a local checkout:

```sh
cargo install --path .
```

## From GitHub Releases

Pre-built binaries are published for every release:

- Linux x86_64 (`portless-linux-amd64.tar.gz`)
- Linux ARM64 (`portless-linux-arm64.tar.gz`)
- macOS Intel (`portless-macos-amd64.tar.gz`)
- macOS Apple Silicon (`portless-macos-arm64.tar.gz`)
- Windows x86_64 (`portless-windows-amd64.zip`)
- Debian package (`portless_*_amd64.deb.gz`)

Download the appropriate archive from
[GitHub Releases](https://github.com/muhammad-fiaz/portless/releases), then:

```sh
# Linux / macOS
tar -xzf portless-linux-amd64.tar.gz
chmod +x portless-linux-amd64
sudo mv portless-linux-amd64 /usr/local/bin/portless
```

```powershell
# Windows (PowerShell)
Expand-Archive -Path portless-windows-amd64.zip -DestinationPath .
Move-Item portless-windows-amd64.exe C:\Windows\System32\portless.exe
```

```sh
# Debian / Ubuntu
tar -xzf portless_*_amd64.deb.gz
sudo dpkg -i portless_*_amd64.deb
```

## Verify

```sh
portless --version
# -> portless 0.0.0
```

## Trust the local CA

```sh
portless trust
```

This adds the locally generated certificate authority to your OS trust store so
that browsers accept `https://*.localhost` certificates without warnings. You
will be prompted for your password (sudo / UAC) on first run.

## Uninstall

```sh
# Stop the proxy and remove CA trust
portless clean

# Remove the binary
cargo uninstall portless    # or `rm /usr/local/bin/portless`
```

## Next

- [Quick Start](./quick-start.md)
- [Why Portless](./why-portless.md)

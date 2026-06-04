<div align="center">

# Portless

**`localhost:3000` → `https://myapp.localhost`.**

[![Crates.io](https://img.shields.io/crates/v/portless.svg)](https://crates.io/crates/portless)
[![Documentation](https://docs.rs/portless/badge.svg)](https://docs.rs/portless)
[![Website](https://img.shields.io/badge/website-muhammad--fiaz.github.io%2Fportless-blue)](https://muhammad-fiaz.github.io/portless)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Build Status](https://github.com/muhammad-fiaz/portless/workflows/CI/badge.svg)](https://github.com/muhammad-fiaz/portless/actions)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)

</div>

> [!TIP]
> **Replacing ports with stable, named `.localhost` URLs for local development.**
> For humans and agents.

```diff
- "dev": "next dev"                    # http://localhost:3000
+ "dev": "portless run next dev"       # https://myapp.localhost
```

> [!NOTE]
> This project concept is inspired by [vercel-labs/portless](https://github.com/vercel-labs/portless).
> Portless is an independent Rust-native reimplementation, not a fork. All credits go to the [Vercel Labs](https://github.com/vercel-labs) team for the original concept and design.

## Why Portless?

Local development with port numbers is fragile:

- **Port conflicts** - two projects default to the same port and you get `EADDRINUSE`.
- **Memorizing ports** - was the API on `3001` or `8080`?
- **Wrong app on refresh** - stop one server, start another on the same port, and your open tab is now a different app.
- **Cookie / storage collisions** - cookies set on `localhost` bleed across apps.
- **Hardcoded ports in config** - CORS allowlists, OAuth redirect URIs, `.env` files all break when ports shift.
- **Agent / AI guess-work** - agents guess or hardcode the wrong port, especially in monorepos.
- **Browser history mess** - `localhost:3000` history is a jumble of unrelated projects.

Portless gives every dev server a stable, named `https://<name>.<tld>` URL.

## Installation

### From crates.io (recommended)

```sh
cargo install portless
```

### From GitHub (latest commit)

```sh
cargo install --git https://github.com/muhammad-fiaz/portless
```

### From a local clone

```sh
git clone https://github.com/muhammad-fiaz/portless
cd portless
cargo install --path .
```

### Pre-built binaries

Download a release binary for your platform from
[Releases](https://github.com/muhammad-fiaz/portless/releases), or use the
one-line installer:

```sh
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/muhammad-fiaz/portless/main/scripts/install.sh | sh

# Windows (PowerShell)
irm https://raw.githubusercontent.com/muhammad-fiaz/portless/main/scripts/install.ps1 | iex
```

## Quick start

```sh
# 1. Trust the local CA (one-time, password may be required)
portless trust

# 2. Run any command through the proxy
portless myapp next dev
# -> https://myapp.localhost

# 3. Or auto-discover the dev script
portless
```

## Inspiration

Inspired by [Portless by Vercel Labs](https://github.com/vercel-labs/portless).

Portless introduced stable localhost development URLs.

This project is an **independent Rust-native implementation** focused on:

- **Reliability** - file-backed route registry, panic isolation, automatic recovery.
- **Concurrency** - Tokio + DashMap + ArcSwap, no global locks in hot paths.
- **Performance** - `release` profile uses LTO, opt-level 3, codegen-units 1.
- **Security** - `#![forbid(unsafe_code)]`, `0600` perms on private keys, SNI cert isolation.
- **Modern networking** - rustls, HTTP/1.1, HTTP/2 with ALPN, optional HTTP/3 (QUIC).
- **Cross-platform** - Linux, macOS, Windows, with native OS service integration.

## Features

- **Stable local URLs** - `https://myapp.localhost` instead of `http://localhost:3000`.
- **Automatic HTTPS** - local CA, per-hostname certs, automatic trust.
- **HTTP/2 by default** - multiplexes requests over a single connection.
- **HTTP/1.1, HTTP/2, HTTP/3, WebSockets, SSE, streaming**.
- **Cross-platform** - Linux, macOS, Windows.
- **Language-agnostic** - runs any executable (`cargo run`, `uv run`, `go run`, `npm run dev`, etc.).
- **Framework-agnostic** - auto-detects Next.js, Vite, Astro, Nuxt, SvelteKit, Angular, etc.
- **Zero-config mode** - `portless` in any project dir.
- **Subdomains** - `https://api.myapp.localhost`.
- **Wildcard routing** - opt-in `tenant1.myapp.localhost` → `myapp.localhost`.
- **Git worktree support** - `fix-ui.myapp.localhost` for linked worktrees.
- **Monorepo support** - pnpm, npm, yarn, bun workspaces.
- **Custom TLD** - `--tld test` (recommended), `--tld internal`, etc.
- **LAN mode** - mDNS `.local` for real-device testing.
- **OS service** - `portless service install` for startup-on-boot.
- **Dashboard** - minimal admin UI on a separate port.
- **Tailscale sharing** - `--tailscale` for tailnet URLs, `--funnel` for public.
- **Auto /etc/hosts sync** - Safari compatibility, opt-out with `PORTLESS_SYNC_HOSTS=0`.
- **Prometheus metrics** - `portless list` and `/api/metrics`.

## Commands

```
portless                                  # Auto: run dev script through proxy
portless run                              # Same as above
portless run [cmd...]                     # Infer name from project
portless <name> <cmd> [args...]           # Run app at https://<name>.<tld>
portless get <name>                       # Print the URL for a service
portless alias <name> <port>              # Static alias (e.g. for Docker)
portless alias --remove <name>            # Remove alias
portless list                             # Show active routes
portless trust                            # Trust local CA
portless untrust                          # Remove trust
portless clean                            # Full reset (state, CA, hosts, service)
portless prune                            # Kill orphan dev servers
portless hosts sync                       # Add routes to /etc/hosts
portless hosts clean                      # Remove from /etc/hosts
portless proxy start                      # Start HTTPS proxy
portless proxy stop                       # Stop proxy
portless proxy status                     # Proxy status
portless service install                  # Install OS startup service
portless service uninstall                # Uninstall service
portless service status                   # Service status
portless version                          # Print version
portless help                             # Show help
```

## Environment variables

| Variable                  | Description                                          | Default            |
|---------------------------|------------------------------------------------------|--------------------|
| `PORTLESS`                | Set to `0` to bypass the proxy                       | enabled            |
| `PORTLESS_PORT`           | Proxy port                                           | `443` / `80`       |
| `PORTLESS_HTTPS`          | `0` to disable HTTPS                                 | enabled            |
| `PORTLESS_LAN`            | `1` to enable LAN mode                               | off                |
| `PORTLESS_LAN_IP`         | Pinned LAN IP                                        | auto               |
| `PORTLESS_TLD`            | Custom TLD (e.g. `test`)                             | `localhost`        |
| `PORTLESS_WILDCARD`       | `1` to enable wildcard subdomain routing             | off                |
| `PORTLESS_SYNC_HOSTS`     | `0` to disable /etc/hosts auto-sync                  | on                 |
| `PORTLESS_TAILSCALE`      | `1` to share on Tailscale                            | off                |
| `PORTLESS_FUNNEL`         | `1` to share via Tailscale Funnel                    | off                |
| `PORTLESS_STATE_DIR`      | Override state directory                             | `~/.portless`      |
| `PORTLESS_APP_PORT`       | Use a fixed port for the child process               | random 4000–4999   |
| `PORTLESS_URL`            | Injected into child: the public URL                  | -                  |
| `PORTLESS_TAILSCALE_URL`  | Injected into child: the Tailscale URL               | -                  |

## Configuration

`portless.json` (or `package.json` `"portless"` key) at the project root:

```json
{
  "name": "myapp",
  "script": "dev",
  "apps": {
    "apps/web": { "name": "web" },
    "apps/api": { "name": "api" }
  }
}
```

| Field      | Type     | Description                                  |
|------------|----------|----------------------------------------------|
| `name`     | string   | Base app name                                |
| `script`   | string   | Default script to run                        |
| `appPort`  | number   | Fixed port for the child process             |
| `proxy`    | boolean  | Whether to route through the proxy           |
| `apps`     | object   | Per-workspace overrides (monorepo)           |
| `turbo`    | boolean  | Use turborepo for multi-app orchestration    |

## Architecture

Portless is a single-crate Rust project. The module tree:

```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Crate root, public API
├── cli/             # Clap subcommand tree
├── proxy/           # HTTPS reverse proxy (hyper, rustls, h2)
├── tls/             # Local CA, per-hostname certs (rcgen)
├── routing/         # Hostname parsing, validation, routing table
├── process/         # Child process management
├── config/          # portless.json, package.json, env vars
├── state/           # Persistent route registry, proxy state
├── dashboard/       # Admin dashboard (HTTP/HTML)
├── service/         # OS service integration (systemd/launchd/Task Scheduler)
├── hosts/           # /etc/hosts synchronization
├── worktree/        # Git worktree detection
├── tailscale/       # Tailscale serve/funnel integration
├── discovery/       # Project & monorepo detection
├── metrics/         # Counters, gauges, histograms
├── platform/        # OS-specific helpers
├── common/          # Shared types, errors, helpers
└── trust.rs         # OS-level CA trust
```

## Cross-platform

| Platform | Service       | Hosts file path                                       |
|----------|---------------|-------------------------------------------------------|
| Linux    | systemd       | `/etc/hosts`                                          |
| macOS    | launchd       | `/etc/hosts`                                          |
| Windows  | Task Scheduler| `%WINDIR%\System32\drivers\etc\hosts`                 |

## Testing

```sh
cargo test
```

## Linting

```sh
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
```

## License

This project is licensed under the Apache-2.0 License. See the [LICENSE](LICENSE) file for details.

## Acknowledgements

- **[Portless by Vercel Labs](https://github.com/vercel-labs/portless)** - the original concept and design that inspired this project. This Rust reimplementation is independent and not a fork.
- The Rust community - for an unparalleled ecosystem of high-quality crates.
- All contributors - see [CONTRIBUTING.md](CONTRIBUTING.md).

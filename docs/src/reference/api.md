# Public API

Portless is a binary-first project, but the core library is also published to
crates.io for embedding into custom tooling.

## Library

```toml
[dependencies]
portless = "0.0"
```

```rust
use portless::state::Registry;

#[tokio::main]
async fn main() -> portless::Result<()> {
    let registry = Registry::in_memory();
    let _ = registry.len();
    Ok(())
}
```

## Modules

- [`cli`](https://docs.rs/portless/latest/portless/cli/) — clap-based command line.
- [`proxy`](https://docs.rs/portless/latest/portless/proxy/) — hyper-util-based HTTPS reverse proxy.
- [`tls`](https://docs.rs/portless/latest/portless/tls/) — local CA and per-hostname certificate generation.
- [`routing`](https://docs.rs/portless/latest/portless/routing/) — hostname parsing, validation, and wildcard matching.
- [`process`](https://docs.rs/portless/latest/portless/process/) — child process supervision and lifecycle.
- [`config`](https://docs.rs/portless/latest/portless/config/) — `portless.json` and `package.json` configuration.
- [`state`](https://docs.rs/portless/latest/portless/state/) — persistent route registry.
- [`dashboard`](https://docs.rs/portless/latest/portless/dashboard/) — admin dashboard.
- [`service`](https://docs.rs/portless/latest/portless/service/) — OS service integration.
- [`hosts`](https://docs.rs/portless/latest/portless/hosts/) — `/etc/hosts` synchronization.
- [`worktree`](https://docs.rs/portless/latest/portless/worktree/) — git worktree detection.
- [`tailscale`](https://docs.rs/portless/latest/portless/tailscale/) — Tailscale serve/funnel integration.
- [`discovery`](https://docs.rs/portless/latest/portless/discovery/) — project and monorepo detection.
- [`metrics`](https://docs.rs/portless/latest/portless/metrics/) — operational metrics.
- [`platform`](https://docs.rs/portless/latest/portless/platform/) — OS-specific helpers.
- [`common`](https://docs.rs/portless/latest/portless/common/) — shared types and errors.

## Crate attributes

```rust
#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(unreachable_pub)]
```

## See also

- [docs.rs/portless](https://docs.rs/portless) — full API reference.
- [Architecture](./architecture.md) — module overview.

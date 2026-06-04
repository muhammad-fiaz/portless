//! # Portless
//!
//! **Rust-native local development platform that replaces localhost port
//! numbers with stable, named HTTPS hostnames.**
//!
//! Instead of juggling port numbers like `http://localhost:3000`,
//! `http://localhost:8080`, and `http://localhost:5173`, portless gives every
//! dev server a memorable name:
//!
//! ```text
//! https://myapp.localhost      → http://127.0.0.1:3000
//! https://api.myapp.localhost  → http://127.0.0.1:8080
//! https://docs.localhost       → http://127.0.0.1:5173
//! ```
//!
//! ## Language Support
//!
//! Portless is **language-agnostic** — it works with any command that starts a
//! TCP server. Common examples:
//!
//! | Language / Runtime | Command |
//! |---|---|
//! | Rust (Cargo) | `portless run cargo run` |
//! | Go | `portless run go run .` |
//! | Python | `portless run uv run main.py` |
//! | Node.js (npm) | `portless run npm run dev` |
//! | Node.js (pnpm) | `portless run pnpm dev` |
//! | Node.js (yarn) | `portless run yarn dev` |
//! | Node.js (bun) | `portless run bun dev` |
//! | Deno | `portless run deno task dev` |
//! | TypeScript (Next.js) | `portless run npx next dev` |
//! | Any process | `portless run <any-command>` |
//!
//! ## Installation
//!
//! ```bash
//! cargo install portless
//! ```
//!
//! On first run, `portless` automatically registers itself on the system PATH
//! so you can invoke it from any directory without a full path.
//!
//! ## Quick Start
//!
//! ```text
//! # Start a named dev server (auto-detects framework)
//! portless run cargo run
//!
//! # Name it explicitly
//! portless myapp cargo run --release
//!
//! # Register an alias for a running service (Docker, etc.)
//! portless alias redis 6379
//!
//! # List active routes
//! portless list
//!
//! # Trust the local CA (run once)
//! portless trust
//! ```
//!
//! ## Architecture
//!
//! | Module | Responsibility |
//! |---|---|
//! | [`cli`] | Clap-based command-line interface and dispatcher |
//! | [`proxy`] | Async reverse proxy (HTTP/1.1, HTTP/2, TLS) |
//! | [`tls`] | Local certificate authority and leaf-cert generation |
//! | [`routing`] | Hostname parsing, validation, and wildcard matching |
//! | [`process`] | Child process supervision and lifecycle |
//! | [`config`] | `portless.json` / `package.json` configuration |
//! | [`state`] | Persistent route registry |
//! | [`dashboard`] | Admin dashboard (feature-gated) |
//! | [`service`] | OS service integration (launchd / systemd / Task Scheduler) |
//! | [`hosts`] | `/etc/hosts` block synchronization |
//! | [`worktree`] | Git worktree detection for branch-scoped names |
//! | [`tailscale`] | Tailscale Serve / Funnel integration |
//! | [`discovery`] | Project, monorepo, framework, and package-manager detection |
//! | [`metrics`] | Operational counters and histograms |
//! | [`platform`] | OS-specific helpers and PATH registration |
//! | [`common`] | Shared error type, result alias, and utilities |
//!
//! ## Example
//!
//! ```no_run
//! use portless::state::Registry;
//!
//! # async fn run() -> portless::common::Result<()> {
//! let registry = Registry::in_memory();
//! let _ = registry.len();
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(unreachable_pub)]

pub mod cli;
pub mod common;
pub mod config;
pub mod dashboard;
pub mod discovery;
pub mod hosts;
pub mod metrics;
pub mod platform;
pub mod process;
pub mod proxy;
pub mod routing;
pub mod service;
pub mod state;
pub mod tailscale;
pub mod tls;
pub mod trust;
pub mod worktree;

pub use common::{Error, Result, Version};

/// Crate version, taken from `Cargo.toml` at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default TLD when none is configured via `PORTLESS_TLD`.
pub const DEFAULT_TLD: &str = "localhost";

/// Default HTTPS port for the proxy (`443`).
pub const DEFAULT_HTTPS_PORT: u16 = 443;

/// Default plain-HTTP port when TLS is disabled (`80`).
pub const DEFAULT_HTTP_PORT: u16 = 80;

/// Lowest port in the dynamic range assigned to child processes (`4000`).
pub const DEFAULT_PORT_RANGE_START: u16 = 4000;

/// Exclusive upper bound of the dynamic port range (`5000`).
pub const DEFAULT_PORT_RANGE_END: u16 = 5000;

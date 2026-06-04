# Development Guide

## Prerequisites

- Latest stable Rust (`rustup default stable`).
- On Linux: `libssl-dev`, `pkg-config`, `build-essential`.
- On macOS: `brew install openssl pkg-config`.
- On Windows: Visual Studio Build Tools 2022 with the "Desktop development
  with C++" workload.

## Setup

```sh
git clone https://github.com/muhammad-fiaz/portless
cd portless
cargo build
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

## Common tasks

### Run the CLI from source

```sh
cargo run -- run npm run dev
```

### Build release binaries

```sh
cargo build --release
# binary at target/release/portless
```

### Cross-compile (Linux only)

```sh
cargo install cross
cross build --release --target aarch64-unknown-linux-musl
```

### Generate docs

```sh
cargo doc --no-deps --all-features --open
mdbook serve docs    # user guide
```

### Run a single test

```sh
cargo test --lib tls::ca::tests::regenerate_changes_fingerprint
```

### Profile with flamegraph

```sh
cargo install flamegraph
cargo flamegraph --bin portless -- run cargo run
```

## CI

CI runs on GitHub Actions:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`
- `cargo build --release`
- `cargo audit`
- `cargo doc --no-deps --all-features`

See `.github/workflows/ci.yml`.

## Debugging tips

- Set `RUST_LOG=portless=debug,tower=info` for verbose logging.
- The proxy logs to `$STATE/logs/proxy.log`.
- Per-child logs are in `$STATE/logs/<hostname>.log`.
- The registry is `$STATE/routes.json` (JSON, human-readable).
- The CA is `$STATE/ca/{ca.crt,ca.key}`.

## Performance

We optimize for cold-start latency, not steady-state throughput. Benchmarks
are in `benches/` and can be run with `cargo bench`.

Key design choices:

- `DashMap` for concurrent route lookup.
- `ArcSwap` for lock-free reads.
- On-disk cert cache for the cold path; in-memory for the hot path.
- TLS handshake offloaded to a connection pool (rustls's `ClientHello` cache).

## Release process

1. Bump version in `Cargo.toml`.
2. Tag the commit: `git tag -s v0.X.Y -m "v0.X.Y"`.
3. Push: `git push origin main --tags`.
4. CI publishes to crates.io and creates a GitHub release with auto-generated
   notes. The release notes are the canonical changelog.

# Contributor Guide

This is the companion to [CONTRIBUTING.md](https://github.com/muhammad-fiaz/portless/blob/main/CONTRIBUTING.md)
and explains the conventions used in this codebase.

## Source layout

The project is a single crate. Modules are under `src/`. Tests live in
`#[cfg(test)] mod tests` blocks at the bottom of each file.

## Coding conventions

- **No `unsafe`.** Period.
- **No `unwrap()` in library code.** Use `?` or `.expect("invariant")` only
  for documented invariants.
- **All public items documented.** Use `///` doc comments.
- **Errors via the `Error` enum.** `From` impls are welcome; custom
  conversions should be in the same module as the source error type.
- **Tracing, not `println!`.** Use `tracing::{info, warn, error, debug}`.
- **No panics in hot paths.** Panics are reserved for impossible states.

## Async

- We use Tokio. `#[tokio::main]`, `#[tokio::test]`.
- Never hold a `parking_lot::MutexGuard` across `.await`. Drop the guard first.
- `tokio::sync::Mutex` is only used when the lock is held across `.await`.
- Prefer `&mut` references over `Arc<Mutex<T>>` where possible.

## Lints

- `cargo fmt --check` must be clean.
- `cargo clippy --all-targets --all-features -- -D warnings` must pass.
- `#![forbid(unsafe_code)]` is enforced at the crate root.

## Adding a new command

1. Add a variant to `cli::opts::CommandKind` (or extend an existing one).
2. Implement the handler as `async fn cmd_<name>(...) -> Result<()>` in
   `cli/commands.rs`.
3. Match on `CommandKind` in the `run` function.
4. Update the README and `docs/user-guide/`.

## Adding a new module

1. Create `src/<module>/mod.rs` (and additional files if needed).
2. Add `pub mod <module>;` to `src/lib.rs`.
3. Add `#[cfg(test)] mod tests;` at the bottom.
4. Document all public items.

## Adding a new dependency

1. Add it to `Cargo.toml`. Use the lowest minor version that has what you
   need.
2. Run `cargo build` to populate `Cargo.lock`.
3. Document why it's needed in a comment near the use site.

## Releases

We follow semver. Breaking changes bump the minor version (we're pre-1.0).
Each release:

1. Bumps the version in `Cargo.toml`.
2. Tags the commit (`git tag v0.X.Y`).
3. CI publishes to crates.io and creates a GitHub release with notes.
4. The release notes become the canonical changelog (we do not maintain a
   separate `CHANGELOG.md`).

## License

By contributing, you agree to license your work under Apache-2.0.

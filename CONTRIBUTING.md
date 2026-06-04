# Contributing to Portless

Thank you for your interest in contributing! Portless is an open-source project
licensed under Apache-2.0, and we welcome contributions of all kinds.

## Code of Conduct

Be kind. Be respectful. Disagree on technical merit. Assume good faith.

## Development setup

1. **Install the latest stable Rust via [rustup](https://rustup.rs) (`rustup default stable`).**

   ```sh
   rustup install stable
   rustup default stable
   ```

2. **Clone the repository:**

   ```sh
   git clone https://github.com/muhammad-fiaz/portless
   cd portless
   ```

3. **Build:**

   ```sh
   cargo build
   ```

4. **Test:**

   ```sh
   cargo test
   ```

5. **Lint and format:**

   ```sh
   cargo fmt
   cargo clippy --all-targets --all-features -- -D warnings
   ```

6. **Generate documentation:**

   ```sh
   cargo doc --no-deps --all-features
   ```

## Code style

- **`#![forbid(unsafe_code)]` is enforced.** No `unsafe` blocks anywhere.
- All public items must have documentation. Use complete sentences.
- Prefer `tracing` over `println!` for runtime output.
- Use `parking_lot::Mutex` / `RwLock` over `std::sync::*` for performance.
- Use `tokio::sync::Mutex` only when holding across an `await` point.
- No panics in library code (`lib.rs`); all public functions return `Result`.
- Tests live in `#[cfg(test)] mod tests` inside each module.

## Commit messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add HTTP/3 support behind a feature flag
fix: handle ECONNRESET on TLS wrapper sockets
docs: clarify wildcard routing in README
refactor: extract hostname parsing to a separate module
test: add round-trip tests for the registry
chore: bump tokio to 1.41
```

## Pull request process

1. Fork the repository.
2. Create a feature branch (`git checkout -b feat/my-feature`).
3. Write your changes and add tests.
4. Ensure `cargo test`, `cargo fmt`, and `cargo clippy` all pass.
5. Update `CHANGELOG.md` with a brief description under the "Unreleased" section.
6. Open a pull request with a clear title and description.

## Architecture decisions

Before making large changes, please open an issue to discuss the design. The
maintainers are happy to talk through trade-offs.

## Release process

1. Update `CHANGELOG.md` with the new version's date and entries.
2. Bump the version in `Cargo.toml`.
3. Tag the commit: `git tag v0.X.Y`.
4. Push the tag: `git push origin v0.X.Y`.
5. CI publishes to crates.io and creates a GitHub release.

## License

By contributing, you agree that your contributions will be licensed under
the Apache-2.0 license. See `LICENSE` for the full text.

## Maintainer

**Muhammad Fiaz** - <contact@muhammadfiaz.com>

- Website: <https://muhammad-fiaz.github.io/portless>
- Repository: <https://github.com/muhammad-fiaz/portless>

## Questions?

Open an issue or start a discussion on GitHub.

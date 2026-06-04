# Acknowledgements

Portless is an independent Rust-native reimplementation of the original
[**vercel-labs/portless**](https://github.com/vercel-labs/portless) idea.

The original concept - replace `localhost:3000` with stable, named
`.localhost` URLs - was pioneered by the
[Vercel Labs](https://github.com/vercel-labs) team. Their TypeScript
implementation is the design reference for this project. All credit for the
idea, the developer experience, and the design philosophy goes to them.

## Differences from the TypeScript version

This Rust reimplementation:

- Has no Node.js runtime dependency.
- Distributes a single static binary per platform.
- Uses the `rcgen` + `rustls` ecosystem for TLS instead of Node's `node-forge`.
- Uses `hyper-util` for the HTTP/1.1 + HTTP/2 reverse proxy instead of Node's
  `http2`.
- Uses `clap` instead of commander.
- Uses `tokio` instead of Node's event loop.
- Implements the same surface area: commands, env vars, config files,
  worktree detection, monorepo support, LAN mode, Tailscale, OS service,
  dashboard, metrics.

## Inspiration

This project is **inspired by** `vercel-labs/portless`. It is **not a fork**.
No source code is shared. The implementation is independent, written from
scratch in Rust, with a different module layout and different internal
architecture choices.

If you are looking for the original TypeScript implementation, please visit
<https://github.com/vercel-labs/portless>.

## Other credits

- The Rust community - for an unparalleled ecosystem of high-quality crates.
- All contributors - see [CONTRIBUTING.md](https://github.com/muhammad-fiaz/portless/blob/main/CONTRIBUTING.md).
- The maintainer, [Muhammad Fiaz](https://muhammad-fiaz.github.io/portless).

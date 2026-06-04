<!--
SEO metadata for this page:

Title: Portless — Rust-native local development platform
Description: Portless is a Rust-native local development platform that gives every dev server a stable https://<name>.<tld> URL. Inspired by vercel-labs/portless.
Author: Muhammad Fiaz <contact@muhammadfiaz.com>
Keywords: portless, localhost, https, proxy, tls, development, rust, vercel-labs
Canonical: https://muhammad-fiaz.github.io/portless/
OG Image: https://muhammad-fiaz.github.io/portless/og-image.png
Twitter Card: summary_large_image
-->

# Portless

> **Replacing ports with stable, named `.localhost` URLs for local development.**
> For humans and agents.

```diff
- "dev": "next dev"                  # http://localhost:3000
+ "dev": "portless run next dev"     # https://myapp.localhost
```

> **This project is inspired by [vercel-labs/portless](https://github.com/vercel-labs/portless).**
> Portless is an independent Rust-native reimplementation, not a fork. All credit for the original concept and design goes to the [Vercel Labs](https://github.com/vercel-labs) team.

## What is Portless?

Portless is a local development platform that gives every dev server a stable,
named URL — for example `https://myapp.localhost` instead of
`http://localhost:3000`. It works with any language, framework, or build tool.

Built in **Rust** for speed, reliability, and a single static binary that
installs anywhere. Inspired by [vercel-labs/portless](https://github.com/vercel-labs/portless).

## Why?

Local development with port numbers is fragile:

- **Port conflicts** — two projects default to the same port and you get `EADDRINUSE`.
- **Memorizing ports** — was the API on `3001` or `8080`?
- **Wrong app on refresh** — stop one server, start another on the same port, and your open tab is now a different app.
- **Monorepo multiplier** — every problem above scales with each service in the repo.
- **Agents test the wrong port** — AI coding agents guess or hardcode the wrong port, especially in monorepos.
- **Cookie / storage collisions** — cookies set on `localhost` bleed across apps.
- **Hardcoded ports in config** — CORS allowlists, OAuth redirect URIs, `.env` files all break when ports shift.
- **Sharing URLs with teammates** — "What port is that on?" becomes a Slack question.
- **Browser history mess** — `localhost:3000` history is a jumble of unrelated projects.

## Quick start

```sh
# 1. Install
cargo install portless

# 2. Trust the local CA (one-time)
portless trust

# 3. Run your dev server
portless run npm run dev
# -> https://myapp.localhost
```

## Features

- 🦀 **Single static Rust binary** — no runtime dependencies.
- 🔒 **HTTPS by default** — auto-generated local CA, HTTP/2 ALPN.
- 🌐 **Custom TLDs** — `.localhost`, `.test`, or your own.
- 🏗️ **Monorepo-aware** — pnpm, npm, yarn, bun workspaces.
- 🌲 **Git worktree integration** — each branch gets its own subdomain.
- 🎯 **Subdomain routing** — `api.myapp.localhost`, `docs.myapp.localhost`, etc.
- 🔁 **Wildcard mode** — `*.myapp.localhost` → `myapp.localhost`.
- 🌎 **LAN mode** — mDNS `.local` for phone/device testing.
- 🛰️ **Tailscale support** — share over your tailnet or expose via Funnel.
- 🛡️ **OS service integration** — systemd, launchd, Task Scheduler.
- 📊 **Metrics & dashboard** — Prometheus metrics, route inspection.
- 🧪 **Framework detection** — Next.js, Vite, Nuxt, Astro, SvelteKit, Remix, etc.

## Inspired by

Portless is an independent Rust-native reimplementation of the original
[**vercel-labs/portless**](https://github.com/vercel-labs/portless) idea. The
TypeScript original pioneered the concept of stable, named `.localhost` URLs
for local development. This project carries that idea into a fast, statically
linked, cross-platform binary written in Rust.

If you are looking for the TypeScript reference implementation, head to
<https://github.com/vercel-labs/portless>.

## License

Apache-2.0 © 2026 [Muhammad Fiaz](https://muhammad-fiaz.github.io/portless).

## Author

**Muhammad Fiaz** — <contact@muhammadfiaz.com>

- Website: <https://muhammad-fiaz.github.io/portless>
- Repository: <https://github.com/muhammad-fiaz/portless>
- Funding: <https://pay.muhammadfiaz.com>

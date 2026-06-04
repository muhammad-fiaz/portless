# Architecture

## Goals

- Replace `localhost:3000` with stable HTTPS hostnames (`https://myapp.localhost`).
- Run as a single user-space binary (no per-app port juggling).
- Cross-platform: Linux, macOS, Windows.
- Safe Rust (`#![forbid(unsafe_code)]`).

## High-level overview

```
                           +-----------------------+
HTTPS request from browser  |                       |
  https://app.localhost:443 |  Proxy (port 443/80)  |   http://127.0.0.1:<port>
        -------------------> |  SNI / Host match     |  ----------------------->   child process
                           |  TLS termination      |
                           |  HTTP/1.1 / HTTP/2    |
                           +-----------+-----------+
                                       |
                                       |   registry.json
                                       v
                           +-----------+-----------+
                           |  Registry             |
                           |  routes.json          |
                           |  (DashMap + ArcSwap)  |
                           +-----------+-----------+
                                       |
                                       |  /etc/hosts sync
                                       v
                           +-----------+-----------+
                           |  Hosts                |
                           |  hosts file           |
                           +-----------------------+
```

The proxy is a long-running process. The CLI spawns child processes (the
user's dev servers) on demand and registers them with the registry. When a
request comes in, the proxy consults the registry, terminates TLS, and
forwards plain HTTP to the child.

## Modules

```
src/
  lib.rs                  crate root
  main.rs                 tokio entry
  cli/                    clap-based command line
  proxy/                  hyper-util-based HTTPS reverse proxy
  tls/                    rcgen + rustls
  routing/                hostname / tld / match
  process/                child management
  config/                 configuration
  state/                  persistent state
  dashboard/              actix-web dashboard
  service/                OS service
  tailscale/              Tailscale serve/funnel
  worktree/               git worktree detection
  hosts/                  /etc/hosts sync
  discovery/              project / framework / package manager detection
  metrics/                counters, gauges, histograms, Prometheus
  platform/               OS paths, privilege, system info
  common/                 shared Error, fs, net, types
  trust.rs                install/uninstall local CA
```

## Concurrency model

- **Tokio multi-thread runtime** with `#[tokio::main]`.
- **`DashMap`** for concurrent route lookup. `ArcSwap` for lock-free reads.
- **`parking_lot::Mutex`** for short critical sections. Never held across `.await`.
- **No `unsafe`**, ever.
- **No `unwrap()`** in library code; tests may.

## TLS

- One local CA per install (`$STATE/ca/{ca.crt,ca.key}`).
- One certificate per registered hostname, generated on first use and cached on disk.
- `rustls` is configured with a custom SNI resolver that pulls from the
  in-memory `CertStore`, falling back to on-disk cache, falling back to live
  generation.
- HTTP/2 is enabled via ALPN; HTTP/1.1 is also supported.

## Routing

1. The proxy reads the `Host:` header.
2. It strips the TLD (default `.localhost`, configurable).
3. It looks up the bare hostname in the in-memory `Registry`.
4. If no match, it falls back to a wildcard table (when `--wildcard` is
   enabled).
5. If still no match, it returns 404 with a friendly HTML page.

## Hosts file

- The proxy writes a single block bounded by `BEGIN PORTLESS` and `END PORTLESS`.
- `portless hosts sync` regenerates the block; `portless hosts clean` removes
  it.
- On Windows we add entries to the in-process hosts file (requires admin to
  write `C:\Windows\System32\drivers\etc\hosts`).

## State directory

State is stored under:

- Linux: `$XDG_DATA_HOME/portless` or `$HOME/.local/share/portless`
- macOS: `$HOME/Library/Application Support/portless`
- Windows: `%APPDATA%\portless`

Override with `PORTLESS_STATE_DIR=/some/path`.

## Errors

A single `Error` enum in `common::error` covers all failure modes. We never
panic in library code; all fallible operations return `Result<T>`. The CLI
converts `Error` into a user-friendly message and an appropriate exit code.

## Security model

- The proxy binds to `0.0.0.0:443` (Linux/macOS) or `127.0.0.1:443` (Windows).
- On Linux, only `CAP_NET_BIND_SERVICE` is required (no root).
- On macOS, port 443 requires `sudo` on first install; we use `pf` to keep
  the low port.
- On Windows, we use `netsh http add urlacl` to bind.
- TLS uses the locally generated CA; the user must trust it (`portless trust`).
- All connections are HTTPS; cleartext is only used for the upstream hop
  (127.0.0.1).

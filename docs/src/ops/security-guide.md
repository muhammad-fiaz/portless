# Security Guide

## Threat model

Portless is a local development tool. We assume:

- The user trusts their own machine.
- The user is the only person connecting to the proxy (or has explicitly
  enabled LAN/Tailscale mode).
- The user has run `portless trust` to install the local CA in their
  browser/OS trust store.

We do **not** defend against:

- A malicious user-mode process on the same machine reading the CA private
  key and impersonating a hostname on the network. (The CA key is `0600` on
  Unix and in the user's profile on Windows.)
- A malicious user-space process on the same machine connecting to
  `127.0.0.1` and bypassing the proxy. (We can't prevent this; nothing can.)

## CA private key

- Located at `$STATE/ca/ca.key`.
- Permissions: `0600` on Unix; ACL-restricted to the current user on Windows.
- Used only to sign per-hostname leaf certificates.
- **Never transmitted** to any remote server.

If the CA key is compromised, run `portless untrust` to remove the trust
anchor, then delete the state directory, then `portless trust` to
re-establish trust with a new key pair.

## Network binding

- The default binds to `127.0.0.1:443` on Windows and `0.0.0.0:443` on Unix.
- On Unix, binding to port 443 requires `CAP_NET_BIND_SERVICE` (or running
  as root). We use `setcap` once on install.
- `PORTLESS_LAN=1` switches the bind to `0.0.0.0` and the proxy accepts
  requests from any local-network peer.

## TLS

- TLS 1.2 and TLS 1.3 are both supported.
- Cipher suites are picked by `rustls`'s default selection, which prefers
  AEAD ciphers (AES-GCM, ChaCha20-Poly1305) and forward secrecy.
- HTTP/2 is enabled via ALPN; HTTP/1.1 is also accepted.
- The SNI resolver pulls certificates from an in-memory cache; cold loads
  come from disk; the cold-of-cold case generates a new leaf cert on the fly.
- All certificates use SHA-256 (SHA-1 is rejected).

## Upstream hop

- The proxy connects to the child process over plain HTTP on `127.0.0.1`.
- We do not add TLS on the upstream hop because the connection never leaves
  the loopback interface.
- The child process is told `http://` (not `https://`) in `PORTLESS_URL` so
  that it doesn't try to verify a self-signed cert.

## Hosts file

- We write a single block bounded by `BEGIN PORTLESS` and `END PORTLESS`.
- We never edit, delete, or rename lines outside that block.
- `portless hosts clean` removes the block.
- `portless hosts sync` regenerates it from the current registry.

## Environment variables

The proxy passes through the user's environment to the child. If you have
secrets in your environment, they are visible to the child. The proxy does
not log them.

## Process management

- Child processes are spawned with `detached: true` on Unix so they get
  their own process group. Signal handlers kill the entire group instead of
  just the immediate child, preventing orphaned dev servers from surviving
  CLI crashes or `kill -9`.
- A loop-detection header (`X-Portless-Hops`) prevents accidental proxy
  loops (e.g. a Vite dev server proxying back through Portless without
  `changeOrigin: true`). Loops respond with 508 Loop Detected.

## Reporting vulnerabilities

Email <contact@muhammadfiaz.com>. We will acknowledge within 48 hours.

For sensitive issues, use our PGP key (TBA) or contact us through
<https://pay.muhammadfiaz.com>.

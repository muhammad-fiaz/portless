# `portless proxy`

## Start

```sh
portless proxy start [flags]
```

The proxy auto-starts on first `portless run`, but you can start it manually
with custom options.

### Flags

| Flag | Description |
|------|-------------|
| `-p, --port <number>` | Proxy port (default: 443, or 80 with `--no-tls`). Auto-elevates with `sudo`. |
| `--no-tls` | Disable HTTPS (use plain HTTP on port 80). |
| `--https` | Enable HTTPS (default, accepted for compatibility). |
| `--lan` | Enable LAN mode (mDNS `.local` domains for real device testing). |
| `--ip <address>` | Override auto-detected LAN IP (use with `--lan`). |
| `--tld <tld>` | Use a custom TLD instead of `.localhost` (e.g. `test`). |
| `--cert <path>` | Custom TLS certificate. |
| `--key <path>` | Custom TLS private key. |
| `--foreground` | Run in foreground instead of daemon mode. |

## Stop

```sh
portless proxy stop
```

## Status

```sh
portless proxy status
```

Reports the current state: running/not-running, port, HTTPS mode, TLD, LAN
mode, PID, and uptime.

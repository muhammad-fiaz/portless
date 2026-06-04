# Commands

This page lists every command in Portless.

## Quick reference

| Command | Description |
|---------|-------------|
| `portless` | Zero-arg mode: run the inferred dev script |
| `portless run` | Run a command through the proxy with inferred name |
| `portless <name>` | Run a command through the proxy with explicit name |
| `portless get` | Print the URL for a service |
| `portless alias` | Register a static alias for an external service |
| `portless list` | List active routes |
| `portless trust` | Trust the local CA in the system trust store |
| `portless untrust` | Remove the local CA from the system trust store |
| `portless clean` | Stop the proxy, remove CA, remove state, remove hosts block |
| `portless prune` | Kill orphaned dev servers from crashed sessions |
| `portless hosts` | Manage `/etc/hosts` synchronization |
| `portless proxy` | Proxy lifecycle (start / stop / status) |
| `portless service` | OS startup service (install / status / uninstall) |
| `portless version` | Print version information |

## Sections

- [Zero-arg mode](./zero-arg-mode.md)
- [`portless run`](./run.md)
- [`portless <name>`](./named.md)
- [`portless get`](./get.md)
- [`portless alias`](./alias.md)
- [`portless list`](./list.md)
- [`portless trust`](./trust.md)
- [`portless clean`](./clean.md)
- [`portless prune`](./prune.md)
- [`portless hosts`](./hosts.md)
- [`portless proxy`](./proxy.md)
- [`portless service`](./service.md)

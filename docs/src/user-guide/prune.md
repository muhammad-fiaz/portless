# `portless prune`

```sh
portless prune [--force]
```

Finds and kills orphaned dev server processes left behind by crashed Portless
sessions. When the CLI is killed with `kill -9` or crashes, child dev servers
may survive and continue holding their ports. This command checks routes
whose owning process is dead but whose port is still in use, terminates the
orphans, and removes the stale route entries. Use `--force` to send `SIGKILL`
instead of `SIGTERM`.

## What it does

1. Read the route registry.
2. For each route, check if the registered PID is still alive.
3. If the PID is dead but something is listening on the route's port:
   - Send `SIGTERM` (or `SIGKILL` with `--force`) to the listener.
   - Remove the stale route entry.
4. Prune Tailscale `serve` registrations left behind by dead CLI sessions.

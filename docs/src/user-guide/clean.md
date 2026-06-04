# `portless clean`

```sh
portless clean
```

Stops the proxy if it is running, removes the Portless CA from the OS trust
store when Portless installed it, deletes allowlisted files under
`~/.portless`, the system state directory, and `PORTLESS_STATE_DIR` when set,
and removes the Portless-managed block from `/etc/hosts`. May prompt for
elevated privileges.

Custom `--cert` / `--key` paths are not removed.

## What gets deleted

- `~/.portless/` (or `$PORTLESS_STATE_DIR/`)
- `routes.json`, `proxy.pid`, `proxy.port`, `proxy.log`, `proxy.lan`
- `ca/ca.crt`, `ca/ca.key`, `certs/`
- Hosts file block (between `BEGIN PORTLESS` and `END PORTLESS` markers)
- OS service (systemd unit, launchd plist, scheduled task)
- Tailscale `serve` registrations created by Portless

## What does NOT get deleted

- Custom certs and keys passed via `--cert` / `--key`
- Project-local files (the project itself, `node_modules/`, `dist/`, etc.)

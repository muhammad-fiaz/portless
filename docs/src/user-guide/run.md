# `portless run`

```sh
portless run [--name <name>] [--script <script>] [--app-port <port>] [--force] [--no-worktree] [cmd] [args...]
```

`portless run` infers the project name from `portless.json`, `package.json`,
git root, or directory name. When no command is given, runs the configured
script (default: `"dev"`) from `package.json`. Use `--name` to override the
inferred name while still applying worktree prefixes.

```sh
portless run                             # run dev script through proxy
portless run next dev                    # infer name from project
portless run --name myapp next dev       # override inferred name
portless run --script start              # run "start" instead of "dev"
portless run --app-port 3000 next dev    # use a fixed port
portless run --force next dev            # override an existing route
portless run --no-worktree next dev      # disable worktree prefix detection
```

## Flags

| Flag | Description |
|------|-------------|
| `--name <name>` | Override the inferred base name (worktree prefix still applies). |
| `--script <name>` | Run a specific `package.json` script instead of the default (`"dev"`). |
| `--app-port <number>` | Use a fixed port for the app instead of auto-assignment. Also configurable via `PORTLESS_APP_PORT` or `portless.json`. |
| `--force` | Override an existing route registered by another process. |
| `--no-worktree` | Disable git worktree prefix detection. |

## Behavior

1. Detect the project (package.json, portless.json, git root).
2. Infer the app name.
3. Detect git worktree (linked worktree → prepend branch name).
4. Find a free port (4000–4999 by default) unless `--app-port` is set.
5. Register a route in the in-memory registry.
6. Auto-start the proxy if not running.
7. Spawn the child process with `PORTLESS_URL`, `PORT`, `HOST` env vars.
8. Wait for the child to exit, then unregister the route.

## Environment

The child process receives:

| Var | Meaning |
|-----|---------|
| `PORTLESS_URL` | Public URL: `https://<name>.<tld>` |
| `PORT` | Local upstream port (auto-assigned or `--app-port`) |
| `HOST` | `127.0.0.1` |
| `PORTLESS_TLD` | TLD (default `localhost`) |
| `PORTLESS_HTTPS` | `1` if HTTPS is enabled |
| `PORTLESS_TAILSCALE_URL` | Tailscale URL (if enabled) |

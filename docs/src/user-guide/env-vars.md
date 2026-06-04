# Environment variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PORTLESS_PORT` | Proxy port | `443` (HTTPS) / `80` (HTTP) |
| `PORTLESS_HTTPS` | HTTPS on by default; set to `0` to disable (same as `--no-tls`) | on |
| `PORTLESS_LAN` | Set to `1` to always enable LAN mode (mDNS `.local` domains) | off |
| `PORTLESS_LAN_IP` | Override auto-detected LAN IP | auto |
| `PORTLESS_TLD` | Use a custom TLD instead of `.localhost` (e.g. `test`) | `localhost` |
| `PORTLESS_WILDCARD` | Enable wildcard subdomain routing | off |
| `PORTLESS_APP_PORT` | Use a fixed port for the app (skip auto-assignment) | random 4000–4999 |
| `PORTLESS_SYNC_HOSTS` | Set to `0` to disable auto-sync of `/etc/hosts` | on |
| `PORTLESS_TAILSCALE` | Enable Tailscale `serve` for this run | off |
| `PORTLESS_FUNNEL` | Enable Tailscale Funnel (implies `--tailscale`) | off |
| `PORTLESS_STATE_DIR` | Override the state directory | `~/.portless` |
| `PORTLESS_FORCE` | Override an existing route on conflict | off |
| `PORTLESS` | Set to `0` to bypass the proxy | enabled |

## State directory

Portless stores state (routes, PID file, port file, TLS marker) in
`~/.portless` on all platforms. Override with `PORTLESS_STATE_DIR`.

### State files

| File | Purpose |
|------|---------|
| `routes.json` | Maps hostnames to ports |
| `routes.lock` | Prevents concurrent writes |
| `proxy.pid` | PID of the running proxy |
| `proxy.port` | Port the proxy is listening on |
| `proxy.log` | Proxy daemon log output |
| `proxy.lan` | Remembers LAN mode and stores the last known LAN IP |

## Port assignment

Apps get a random port in the 4000–4999 range. Portless sets `PORT` and
usually `HOST` before running your command. Most frameworks respect `PORT`
automatically. For frameworks that ignore it (Vite, Astro, React Router,
Angular, Expo, React Native), Portless auto-injects the right `--port` flag
and, when needed, a matching `--host` flag.

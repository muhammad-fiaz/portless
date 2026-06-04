# Tailscale

Share any Portless app over your Tailscale network with zero framework
config. Each app is root-mounted on its own Tailscale HTTPS port (443, 8443,
8444, ...) so no basePath configuration is needed.

```sh
portless myapp --tailscale next dev
# -> https://myapp.tail1234.ts.net
```

Or globally:

```sh
PORTLESS_TAILSCALE=1 portless myapp next dev
```

## Funnel

Expose apps to the public internet through Tailscale Funnel:

```sh
portless myapp --funnel next dev
# -> https://myapp.tail1234.ts.net
# (publicly accessible)
```

`--funnel` implies `--tailscale`. Also configurable via `PORTLESS_FUNNEL=1`.

## Environment

When Tailscale is active, the child process receives `PORTLESS_TAILSCALE_URL`
containing the Tailscale HTTPS URL, so apps can self-reference their public
address.

## Cleanup

`portless prune` removes orphaned Tailscale `serve` entries left behind by
dead CLI sessions. `portless clean` tears down all Tailscale `serve`/`funnel`
registrations alongside the usual CA and hosts-file cleanup.

# `portless alias`

```sh
portless alias <name> <port> [--force]
portless alias --remove <name>
```

Register a route for a service not managed by Portless (e.g. a Docker
container with a published port). Aliases persist across stale-route cleanup.

```sh
portless alias my-postgres 5432     # -> https://my-postgres.localhost
portless alias redis 6379           # -> https://redis.localhost
portless alias --remove my-postgres # remove the alias
```

## How it works

- The alias is a static route: `https://<name>.<tld>` → `127.0.0.1:<port>`.
- Unlike `portless run`, the alias does not own a process. If nothing is
  listening on `<port>`, the proxy returns 502.
- Aliases survive `portless prune` and `portless clean` only when explicitly
  re-added.

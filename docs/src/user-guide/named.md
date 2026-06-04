# `portless <name>`

```sh
portless <name> [--app-port <port>] [--force] [cmd] [args...]
portless <subdomain>.<name> ...    # use a subdomain
```

`portless <name>` uses an explicit name with no inference or prefixing.

```sh
portless myapp next dev            # explicit name
portless api pnpm start            # service named "api"
portless docs.myapp next dev       # subdomain "docs" of "myapp"
```

## Subdomains

You can split your app into multiple services using subdomains:

```sh
portless api.myapp pnpm start
# -> https://api.myapp.localhost

portless docs.myapp next dev
# -> https://docs.myapp.localhost
```

## Subdomain routing

By default, subdomains do **not** match parent hostnames. So
`api.myapp.localhost` will not route to `myapp.localhost` automatically. Use
the `--wildcard` flag (or `PORTLESS_WILDCARD=1`) to restore the older
behaviour where any subdomain forwards to the registered parent.

## Flags

| Flag | Description |
|------|-------------|
| `--app-port <number>` | Use a fixed port for the app instead of auto-assignment. |
| `--force` | Override an existing route registered by another process. |

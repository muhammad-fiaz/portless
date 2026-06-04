# Subdomains

Organize services with subdomains:

```sh
portless api.myapp pnpm start
# -> https://api.myapp.localhost

portless docs.myapp next dev
# -> https://docs.myapp.localhost
```

Each subdomain gets its own cert, its own route, and its own scope for
cookies and `localStorage`.

## Routing

By default, subdomains are routed **only to themselves**. So
`api.myapp.localhost` will not match a route registered for `myapp.localhost`.
Use the `--wildcard` flag (or `PORTLESS_WILDCARD=1`) to opt into the older
"subdomain falls back to parent" behavior.

## Cookie isolation

Each `.localhost` subdomain is its own origin. Cookies set on
`api.myapp.localhost` are not sent to `docs.myapp.localhost`. This is exactly
what you want for multi-service development.

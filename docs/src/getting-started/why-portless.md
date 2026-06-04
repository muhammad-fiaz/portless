# Why Portless?

Local development with port numbers is fragile. Portless fixes that by giving
each dev server a stable, named URL.

## The problems

### Port conflicts

Two projects default to the same port and you get `EADDRINUSE`. With Portless,
each app gets an auto-assigned port behind a named URL. No collisions.

### Memorizing ports

Was the API on `3001` or `8080`? With Portless, it is always
`https://api.localhost`.

### Wrong app on refresh

Stop one server, start another on the same port, and your open tab shows
something completely different. Named URLs eliminate this.

### Monorepo multiplier

Every problem above scales with each service in the repo. Portless handles any
number of services with distinct hostnames.

### Agents test the wrong port

AI coding agents guess or hardcode the wrong port, especially in monorepos. A
stable URL like `https://myapp.localhost` is deterministic and reliable.

### Cookie and storage clashes

Cookies set on `localhost` bleed across apps on different ports. `localStorage`
is lost when ports shift. Each `.localhost` subdomain gets its own scope.

### Hardcoded ports in config

CORS allowlists, OAuth redirect URIs, and `.env` files all break when ports
change. With Portless, URLs are stable across restarts.

### Sharing URLs with teammates

"What port is that on?" becomes a Slack question. With Portless, everyone uses
the same named URL.

### Browser history mess

Your history for `localhost:3000` is a jumble of unrelated projects. Named URLs
keep things organized.

## The fix

Replace every `http://localhost:<port>` with a stable, named
`https://<name>.<tld>` URL:

```diff
- "dev": "next dev"                 # http://localhost:3000
+ "dev": "portless run next dev"    # https://myapp.localhost
```

## Next

- [Installation](./installation.md)
- [Quick Start](./quick-start.md)
- [Commands](../user-guide/commands.md)

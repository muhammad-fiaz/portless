# `portless list`

```sh
portless list
```

Shows active routes and their assigned ports:

```
HOSTNAME                          PORT    PID      COMMAND
api.localhost                     3001    12345    npm run dev
web.localhost                     3000    12346    next dev
```

## Tailscale URL column

When Tailscale sharing is active, an extra column shows the tailnet URL:

```
HOSTNAME                          PORT    PID      COMMAND                TAILSCALE
api.localhost                     3001    12345    npm run dev            https://api.tail1234.ts.net
```

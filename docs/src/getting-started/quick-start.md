# Quick Start

## Install

```sh
cargo install portless
```

Or download a pre-built binary from
[GitHub Releases](https://github.com/muhammad-fiaz/portless/releases).

## Trust the local CA

```sh
portless trust
```

This is a one-time step. It installs the locally generated certificate
authority in your OS trust store so that browsers accept
`https://*.localhost` certificates without warnings.

## Run your dev server

```sh
cd my-app
portless run npm run dev
# -> https://my-app.localhost
```

That's it! Your dev server is now accessible at a stable HTTPS URL.

## What happens behind the scenes

1. Portless spawns your dev server on a free port (4000–4999 by default).
2. It registers a route in the in-memory registry.
3. It syncs the new hostname to `/etc/hosts` (unless `PORTLESS_SYNC_HOSTS=0`).
4. The proxy is auto-started on port 443 (HTTPS by default).
5. Your browser requests `https://my-app.localhost` and gets forwarded to the
   child process.

## Next steps

- Learn about [all commands](../user-guide/commands.md).
- Set up [HTTPS](../user-guide/https.md).
- Read about [configuration](../user-guide/configuration.md).
- Check the [architecture](../reference/architecture.md).

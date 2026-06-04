# Configuration

Bare `portless` works out of the box. It runs the `dev` script from
`package.json` through the proxy, inferring the app name from the package
name, git root, or directory.

```sh
portless        # runs "dev" script, https://<project>.localhost
```

## Topics

- [`portless.json`](./portless-json.md)
- [Monorepo](./monorepo.md)
- [Subdomains](./subdomains.md)
- [Git worktrees](./worktrees.md)
- [Custom TLD](./custom-tld.md)
- [Wildcard routing](./wildcard.md)
- [LAN mode](./lan.md)
- [Tailscale](./tailscale.md)
- [Environment variables](./env-vars.md)

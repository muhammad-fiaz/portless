# Configuration reference

## `portless.json`

```json
{
  "name": "myapp",
  "script": "dev",
  "appPort": 3000,
  "proxy": true,
  "turbo": true,
  "apps": {
    "apps/web": { "name": "web" },
    "apps/api": { "name": "api.web" }
  }
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | inferred | Base app name. |
| `script` | string | `"dev"` | `package.json` script to run. |
| `appPort` | number | auto | Fixed port for the child process. |
| `proxy` | boolean | auto | Whether to route through the proxy. |
| `turbo` | boolean | `true` | Use turborepo for monorepos. |
| `apps` | object | | Per-package overrides. |

## `package.json` `"portless"` key

```json
{
  "name": "@myorg/web",
  "portless": { "name": "myapp", "script": "dev:app" }
}
```

## Environment variables

See [Environment variables](../user-guide/env-vars.md) for the full table.

## State directory

| File | Purpose |
|------|---------|
| `routes.json` | Maps hostnames to ports |
| `routes.lock` | Prevents concurrent writes |
| `proxy.pid` | PID of the running proxy |
| `proxy.port` | Port the proxy is listening on |
| `proxy.log` | Proxy daemon log output |
| `proxy.lan` | Remembers LAN mode and stores the last known LAN IP |
| `ca/ca.crt` | Local CA certificate |
| `ca/ca.key` | Local CA private key (mode 0600) |
| `certs/<hostname>.crt` | Per-hostname certificate |
| `certs/<hostname>.key` | Per-hostname private key |
| `logs/<hostname>.log` | Per-child stdout/stderr |

# `portless.json`

Use an optional `portless.json` to override defaults:

```json
{ "name": "myapp" }
```

```sh
portless        # runs "dev" script, https://myapp.localhost
```

The name is inferred from `package.json` if not set in config. The script
defaults to `"dev"`.

## Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | inferred | Base app name. Worktree prefix still applies. |
| `script` | string | `"dev"` | Name of a `package.json` script to run. |
| `appPort` | number | auto | Fixed port for the child process. |
| `proxy` | boolean | auto | Whether to route through the proxy. Auto-detected from the command. Set `false` for non-server scripts. |
| `apps` | object | | Overrides for workspace packages, keyed by relative path. |
| `turbo` | boolean | `true` | Set `false` to use direct spawning instead of turborepo in multi-app mode. |

Each `apps` entry has the same shape (`name`, `script`, `appPort`, `proxy`).
When `apps` is present, top-level fields apply only in single-app mode.

## `package.json` `"portless"` key

Instead of a separate `portless.json`, you can add a `"portless"` key to your
`package.json`. A string value is shorthand for setting the name:

```json
{
  "name": "@myorg/web",
  "portless": "myapp"
}
```

An object supports all per-app fields (`name`, `script`, `appPort`, `proxy`):

```json
{
  "name": "@myorg/web",
  "portless": { "name": "myapp", "script": "dev:app" }
}
```

The `package.json` `"portless"` key takes precedence over `portless.json` app
entries but is overridden by CLI flags.

## Precedence

Closest config wins, like Prettier and ESLint:

- For `name`: CLI `--name` > `package.json` `"portless"` > `portless.json` app entry > `package.json` inference.
- For `script`: CLI `--script` > `package.json` `"portless"` > `portless.json` app entry > default `"dev"`.
- For `appPort`: CLI `--app-port` > `PORTLESS_APP_PORT` > `package.json` `"portless"` > `portless.json` app entry > auto-assigned.

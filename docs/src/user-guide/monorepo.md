# Monorepo

One `portless.json` at the repo root covers all workspace packages. Portless
discovers packages from `pnpm-workspace.yaml`, or the `"workspaces"` field in
`package.json` (npm, yarn, bun):

```json
{
  "apps": {
    "apps/web": { "name": "myapp" },
    "apps/api": { "name": "api.myapp" }
  }
}
```

```sh
portless                  # from repo root: start all packages with a "dev" script
cd apps/web && portless   # start just one package
portless --script start   # run "start" instead of "dev"
```

The `apps` map is optional and only needed for overrides. Packages not listed
still auto-discover with names inferred from their `package.json`.

## Hostname convention

Without an `apps` map, hostnames follow the `<package>.<project>.localhost`
convention. The project name comes from the most common npm scope across
workspace packages (e.g. `@myorg/web` and `@myorg/api` produce project name
`myorg`), falling back to the workspace root directory name.

If a package's short name matches the project name, it gets the bare
`<project>.localhost` without duplication.

## Turborepo

For turborepo projects, use Portless as the dev script and the real command
in a separate script:

```json
{
  "scripts": {
    "dev": "portless",
    "dev:app": "next dev"
  },
  "portless": { "name": "myapp", "script": "dev:app" }
}
```

`pnpm dev` at the root runs turbo, which runs Portless in each package.
Portless detects the package manager and runs `pnpm run dev:app` through the
proxy. No `turbo.json` changes are needed.

People without Portless installed can run `pnpm run dev:app` directly.

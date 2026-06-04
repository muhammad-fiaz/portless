# Zero-arg mode

Run bare `portless` from any project directory. It reads the `dev` script
from `package.json` and runs it through the proxy. The app name is inferred
from the package name, git root, or directory name. From a monorepo root
(detected via `pnpm-workspace.yaml` or `package.json` `"workspaces"` field),
Portless starts all workspace packages that have the target script.

```sh
portless                      # single app: run dev script
portless                      # monorepo root: start all apps
portless --script start       # run "start" instead of "dev"
```

## Override defaults

Use a `portless.json` to override defaults. See
[Configuration](../user-guide/configuration.md).

## Inference order

The app name is inferred in this order (first match wins):

1. `--name` CLI flag
2. `package.json` `"portless"` key
3. `portless.json` `name` field
4. `package.json` `name` field (with npm-scope stripping)
5. Git root directory name
6. Current working directory name

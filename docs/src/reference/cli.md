# CLI reference

```
portless [--verbose] [--state-dir <path>] [--bypass] [command]
```

## Global flags

| Flag | Description |
|------|-------------|
| `-v, --verbose` | Increase verbosity (`-vv` for trace). |
| `--state-dir <path>` | Override the state directory. |
| `--bypass` | Bypass the proxy (run command directly). |

## Subcommands

| Command | Description |
|---------|-------------|
| `run` | Run a command through the proxy with inferred name. |
| `named` | Run a named app through the proxy: `portless <name> <command...>`. |
| `get` | Print the URL for a service. |
| `alias` | Register a static alias (e.g. for a Docker service). |
| `list` | List active routes. |
| `trust` | Trust the local CA in the system trust store. |
| `untrust` | Remove the local CA from the system trust store. |
| `clean` | Remove state, CA trust entry, and hosts block. |
| `prune` | Kill orphaned dev servers from crashed sessions. |
| `hosts` | Manage `/etc/hosts` synchronization. |
| `proxy` | Proxy control. |
| `service` | Install/uninstall/status the OS startup service. |
| `version` | Print version information. |

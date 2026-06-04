# Git worktrees

`portless run` automatically detects git worktrees. In a linked worktree, the
branch name is prepended as a subdomain so each worktree gets its own URL
without any config changes.

```sh
# Main worktree (no prefix)
portless run next dev   # -> https://myapp.localhost

# Linked worktree on branch "fix-ui"
portless run next dev   # -> https://fix-ui.myapp.localhost
```

Put `portless run` in your `package.json` once and it works everywhere. The
main checkout uses the plain name, each worktree gets a unique subdomain.
No collisions, no `--force`.

## Override the base name

Use `--name` to override the inferred base name while keeping the worktree
prefix:

```sh
portless run --name myapp next dev
# -> https://fix-ui.myapp.localhost
```

## DNS label length

Worktree-prefixed hostnames are truncated to respect the 63-character DNS
label limit.

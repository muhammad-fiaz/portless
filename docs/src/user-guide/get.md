# `portless get`

```sh
portless get <name> [--no-worktree]
```

Print the URL for a service. Useful for wiring services together in scripts
or environment variables:

```sh
BACKEND_URL=$(portless get backend)
echo "Backend is at: $BACKEND_URL"
```

Applies worktree prefix detection by default. Use `--no-worktree` to skip it.

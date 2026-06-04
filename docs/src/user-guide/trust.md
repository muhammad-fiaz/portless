# `portless trust`

```sh
portless trust
```

Adds the Portless certificate authority to your system trust store. Required
once for HTTPS with auto-generated certificates.

- macOS: adds to the System keychain (requires `sudo`).
- Linux: copies to `/usr/local/share/ca-certificates/` (or distro equivalent)
  and runs `update-ca-certificates`.
- Windows: adds to the ROOT store via `certutil`.

If the CA is already trusted, the command is a no-op.

## Untrust

```sh
portless untrust
```

Removes the local CA from the system trust store.

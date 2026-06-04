# Custom TLD

By default, Portless uses `.localhost` which auto-resolves to `127.0.0.1` in
most browsers. If you prefer a different TLD (e.g. `.test`), use `--tld`:

```sh
portless proxy start --tld test
portless myapp next dev
# -> https://myapp.test
```

The proxy auto-syncs `/etc/hosts` for route hostnames, so `.test` and other
dev hostnames resolve on your machine.

## Recommendations

- **Recommended**: `.test` (IANA-reserved, no collision risk).
- **Avoid**: `.local` (conflicts with mDNS/Bonjour).
- **Avoid**: `.dev` (Google-owned, forces HTTPS via HSTS - that one is
  actually a feature, but collisions with real Google services are possible).

## TLD-specific warnings

`portless proxy start --tld local` prints a warning. `portless proxy start
--tld dev` is allowed but also warned about. You can suppress the warning
with `--no-warn-tld`.

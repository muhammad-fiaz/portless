# Wildcard routing

By default, subdomains are routed **only to themselves**. To restore the
older behavior where any subdomain forwards to a registered parent hostname,
enable wildcard routing:

```sh
portless run --wildcard next dev
# any subdomain of myapp.localhost is forwarded:
#   https://tenant-a.myapp.localhost
#   https://tenant-b.myapp.localhost
```

Or via environment variable:

```sh
PORTLESS_WILDCARD=1 portless run next dev
```

Exact matches take priority over wildcards.

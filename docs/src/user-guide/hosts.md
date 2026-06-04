# `portless hosts`

```sh
portless hosts sync     # add current routes to /etc/hosts
portless hosts clean    # remove Portless entries from /etc/hosts
portless hosts status   # show current hosts file block
```

Auto-sync is **on** by default. Set `PORTLESS_SYNC_HOSTS=0` to disable.

## Why this exists

Some platforms (Safari, certain Linux distros) do not resolve `*.localhost` to
`127.0.0.1` automatically. Portless writes a single block to `/etc/hosts`
bounded by `BEGIN PORTLESS` and `END PORTLESS` markers, so it is easy to
identify and remove our entries.

## Block format

```
# BEGIN PORTLESS (managed by portless; do not edit)
127.0.0.1 myapp.localhost
127.0.0.1 api.myapp.localhost
# END PORTLESS
```

The block is regenerated on every route change, so it always reflects the
current set of registered hostnames.

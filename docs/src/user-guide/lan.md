# LAN mode

Access services from phones and other devices on the same WiFi via mDNS
`.local` hostnames:

```sh
portless proxy start --lan
portless proxy start --lan --https
portless proxy start --lan --ip 192.168.1.42   # manual IP override
```

Make it permanent by adding `export PORTLESS_LAN=1` to your shell profile.
Portless also remembers LAN mode via `proxy.lan`, so a stopped LAN proxy
starts in LAN mode again unless you override it with `PORTLESS_LAN=0` for
that start.

## Implementation

Uses `dns-sd` on macOS (built-in) and `avahi-publish-address` on Linux
(`avahi-utils`). Not supported on Windows.

## Framework notes

### Next.js

Add `allowedDevOrigins: ['myapp.local', '*.myapp.local']` to `next.config.js`
so LAN mode requests still work when Portless adds a git worktree prefix.

### Vite / React Router / SvelteKit / Astro

Handled automatically via `__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS`.

### Expo / React Native

Add `NSAllowsLocalNetworking` to your `app.json` for iOS ATS:

```json
{
  "expo": {
    "ios": {
      "infoPlist": {
        "NSAppTransportSecurity": {
          "NSAllowsLocalNetworking": true
        }
      }
    }
  }
}
```

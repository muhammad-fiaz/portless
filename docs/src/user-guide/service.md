# `portless service`

```sh
portless service install
portless service status
portless service uninstall
```

Installs the proxy as an OS startup service. The default is HTTPS on port 443
with `.localhost` names, but `service install` accepts proxy options
including `--port`, `--no-tls`, `--lan`, `--ip`, `--tld`, `--wildcard`,
`--cert`, and `--key`. Use `--state-dir <path>` or `PORTLESS_STATE_DIR=<path>`
to choose where service state and logs are written.

```sh
portless service install --lan
portless service install --wildcard
PORTLESS_STATE_DIR=~/.portless-lan PORTLESS_LAN=1 portless service install
```

The chosen service configuration is written into `launchd`, `systemd`, or
Task Scheduler and reused after reboot. `portless service status` reports the
installed port, HTTPS mode, TLD, LAN mode, wildcard mode, and state
directory. macOS and Linux use a root-owned service so port 443 can bind at
boot. Windows uses a Task Scheduler startup task that runs as SYSTEM.

Installation and removal may require administrator privileges.
`portless clean` automatically removes the service.

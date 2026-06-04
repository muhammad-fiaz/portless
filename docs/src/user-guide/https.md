# HTTPS

HTTP/2 + TLS is enabled by default for faster dev server page loads. Browsers
limit HTTP/1.1 to 6 connections per host, which bottlenecks dev servers
serving many unbundled files. HTTP/2 multiplexes all requests over a single
connection.

First run generates a local CA and server certs, then adds the CA to your
system trust store. After that, no prompts, no browser warnings.

## Custom certificates

Use your own certs (e.g. from `mkcert`):

```sh
portless proxy start --cert ./cert.pem --key ./key.pem
```

## Trust the CA later

If you skipped the trust prompt on first run:

```sh
portless trust
```

## Disable HTTPS

Use `--no-tls` to run with plain HTTP on port 80:

```sh
portless proxy start --no-tls
```

## HTTP-to-HTTPS redirect

When the HTTPS proxy runs on port 443, a companion HTTP server on port 80
automatically redirects all requests to HTTPS. Set `PORTLESS_HTTPS=0` (or
`--no-tls`) to disable both.

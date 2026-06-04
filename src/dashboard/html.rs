//! Minimal HTML for the dashboard.

/// Render the dashboard index page.
pub fn index() -> String {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Portless Dashboard</title>
<style>
:root { color-scheme: light dark; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; margin: 0; padding: 0; background: #f8fafc; color: #0f172a; }
header { background: #0f172a; color: #f8fafc; padding: 1rem 2rem; }
h1 { margin: 0; font-size: 1.25rem; }
main { max-width: 1024px; margin: 0 auto; padding: 1.5rem; }
section { background: #fff; border-radius: 0.5rem; padding: 1.5rem; margin-bottom: 1rem; box-shadow: 0 1px 2px rgba(0,0,0,0.04); }
h2 { margin-top: 0; font-size: 1.1rem; }
table { border-collapse: collapse; width: 100%; font-size: 0.9rem; }
th, td { padding: 0.5rem 0.75rem; text-align: left; border-bottom: 1px solid #e2e8f0; }
th { background: #f1f5f9; font-weight: 600; }
.muted { color: #64748b; }
.metric { display: inline-block; padding: 0.5rem 1rem; background: #eef2ff; color: #312e81; border-radius: 0.375rem; margin-right: 0.5rem; margin-bottom: 0.5rem; font-variant-numeric: tabular-nums; }
</style>
</head>
<body>
<header><h1>Portless</h1></header>
<main>
<section>
<h2>Routes</h2>
<table id="routes"><thead><tr><th>Hostname</th><th>Port</th><th>PID</th><th>Command</th><th>Started</th></tr></thead><tbody></tbody></table>
</section>
<section>
<h2>Metrics</h2>
<div id="metrics"></div>
</section>
<section>
<h2>Proxy</h2>
<pre id="proxy" class="muted">Loading…</pre>
</section>
</main>
<script>
async function refresh() {
  const r = await fetch('/api/status');
  if (!r.ok) return;
  const j = await r.json();
  const tbody = document.querySelector('#routes tbody');
  tbody.innerHTML = '';
  for (const route of j.routes) {
    const tr = document.createElement('tr');
    tr.innerHTML = `<td>${route.hostname}</td><td>${route.port}</td><td>${route.pid || ''}</td><td>${(route.command || '')}</td><td>${route.started_at}</td>`;
    tbody.appendChild(tr);
  }
  const m = j.metrics;
  document.getElementById('metrics').innerHTML =
    `<span class="metric">Requests: ${m.requests_total}</span>` +
    `<span class="metric">2xx/3xx: ${m.responses_2xx_3xx}</span>` +
    `<span class="metric">4xx: ${m.responses_4xx}</span>` +
    `<span class="metric">5xx: ${m.responses_5xx}</span>` +
    `<span class="metric">In flight: ${m.in_flight}</span>` +
    `<span class="metric">TLS: ${m.tls_handshakes}</span>` +
    `<span class="metric">Uptime: ${m.uptime_secs.toFixed(1)}s</span>`;
  document.getElementById('proxy').textContent = j.proxy ? JSON.stringify(j.proxy, null, 2) : 'not running';
}
refresh();
setInterval(refresh, 5000);
</script>
</body>
</html>"#;
    html.to_string()
}

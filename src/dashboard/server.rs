//! Dashboard HTTP server (separate port from the main proxy).

use crate::common::Result;
use crate::dashboard::api::status;
use crate::dashboard::html::index as render_index;
use crate::metrics::Metrics;
use crate::routing::match_::Router;
use crate::state::proxy_state::ProxyState;
use std::net::SocketAddr;
use std::sync::Arc;

/// Configuration for the dashboard server.
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    /// Bind address.
    pub bind: SocketAddr,
}

/// The dashboard server.
pub struct DashboardServer {
    pub(crate) config: DashboardConfig,
    pub(crate) router: Arc<Router>,
    pub(crate) metrics: Arc<Metrics>,
}

impl std::fmt::Debug for DashboardServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DashboardServer")
            .field("config", &self.config)
            .finish()
    }
}

impl DashboardServer {
    /// Construct a new dashboard server.
    pub fn new(config: DashboardConfig, router: Arc<Router>, metrics: Arc<Metrics>) -> Self {
        Self {
            config,
            router,
            metrics,
        }
    }

    /// Run the dashboard server until a shutdown signal is received.
    pub async fn run(self) -> Result<()> {
        use tokio::net::TcpListener;
        let listener = TcpListener::bind(self.config.bind).await?;
        tracing::info!("dashboard listening on http://{}", self.config.bind);
        loop {
            let (stream, _peer) = listener.accept().await?;
            let router = self.router.clone();
            let metrics = self.metrics.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_req(stream, router, metrics).await {
                    tracing::debug!("dashboard conn ended: {e}");
                }
            });
        }
    }
}

async fn handle_req(
    stream: tokio::net::TcpStream,
    router: Arc<Router>,
    metrics: Arc<Metrics>,
) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut s = stream;
    let mut buf = vec![0u8; 8192];
    let n = s.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }
    let req = String::from_utf8_lossy(&buf[..n]).to_string();
    let first_line = req.lines().next().unwrap_or_default();
    let path = first_line.split_whitespace().nth(1).unwrap_or("/");
    let (status_line, body, content_type) = match path {
        "/" | "/index.html" => (
            "HTTP/1.1 200 OK",
            render_index(),
            "text/html; charset=utf-8",
        ),
        "/api/status" => {
            let routes = router.list();
            let proxy: Option<ProxyState> = None; // populated by the proxy
            let body = serde_json::to_string(&status(routes, proxy, &metrics))
                .unwrap_or_else(|_| "{}".to_string());
            ("HTTP/1.1 200 OK", body, "application/json; charset=utf-8")
        }
        "/api/metrics" => (
            "HTTP/1.1 200 OK",
            metrics.render_prometheus(),
            "text/plain; version=0.0.4",
        ),
        "/healthz" => (
            "HTTP/1.1 200 OK",
            "ok".to_string(),
            "text/plain; charset=utf-8",
        ),
        _ => (
            "HTTP/1.1 404 Not Found",
            "not found".to_string(),
            "text/plain; charset=utf-8",
        ),
    };
    let resp = format!(
        "{status_line}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    s.write_all(resp.as_bytes()).await?;
    s.shutdown().await.ok();
    Ok(())
}

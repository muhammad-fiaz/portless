//! Proxy server: ties together TLS, HTTP/1.1, HTTP/2, optional HTTP/3, and
//! graceful shutdown.

use crate::common::{Error, Result};
use crate::config::Env;
use crate::platform::Paths;
use crate::proxy::acceptor::Acceptor;
use crate::proxy::handler::Handler;
use crate::routing::match_::Router;
use crate::routing::tld::Tld;
use crate::state::proxy_state::ProxyState;
use crate::tls::Ca;
use crate::tls::CertLoader;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;

/// Configuration for the proxy server.
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// Bind address.
    pub bind: SocketAddr,
    /// Whether to enable HTTPS.
    pub https: bool,
    /// TLD in use.
    pub tld: Tld,
    /// Wildcard routing.
    pub wildcard: bool,
    /// Custom cert path (overrides CA).
    pub cert: Option<std::path::PathBuf>,
    /// Custom key path (overrides CA).
    pub key: Option<std::path::PathBuf>,
}

impl ProxyConfig {
    /// Construct from environment variables.
    pub fn from_env() -> Result<Self> {
        let env = Env::load();
        let port = env.port().unwrap_or(443);
        let https = !env.https_disabled();
        let tld = Tld::new(env.tld().unwrap_or("localhost"))?;
        let wildcard = env.wildcard();
        let bind: SocketAddr = format!("0.0.0.0:{port}")
            .parse()
            .map_err(|e: std::net::AddrParseError| Error::proxy(e.to_string()))?;
        Ok(Self {
            bind,
            https,
            tld,
            wildcard,
            cert: env.cert(),
            key: env.key(),
        })
    }
}

/// The proxy server.
pub struct ProxyServer {
    config: ProxyConfig,
    paths: Paths,
    router: Arc<Router>,
    handler: Handler,
    cert_loader: Option<Arc<CertLoader>>,
    acceptor: Option<Acceptor>,
}

impl std::fmt::Debug for ProxyServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyServer")
            .field("config", &self.config)
            .finish()
    }
}

impl ProxyServer {
    /// Construct a new proxy server.
    pub async fn new(config: ProxyConfig, paths: Paths, router: Arc<Router>) -> Result<Self> {
        let handler = Handler::new(router.clone());
        let mut server = Self {
            config,
            paths,
            router,
            handler,
            cert_loader: None,
            acceptor: None,
        };
        if server.config.https {
            let ca = Arc::new(Ca::open(&server.paths).await?);
            let loader = Arc::new(CertLoader::new(ca, server.paths.clone()));
            let acceptor = Acceptor::new(loader.clone())?.with_http2(loader.clone());
            server.cert_loader = Some(loader);
            server.acceptor = Some(acceptor);
        }
        server.router.set_tld(server.config.tld.clone());
        server.router.set_wildcard(server.config.wildcard);
        Ok(server)
    }

    /// Run the server until a shutdown signal is received.
    pub async fn run(self) -> Result<()> {
        let listener = TcpListener::bind(self.config.bind).await?;
        tracing::info!("portless proxy listening on {}", self.config.bind);
        let shutdown = shutdown_signal();
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                _ = &mut shutdown => {
                    tracing::info!("shutdown signal received; stopping proxy");
                    return Ok(());
                }
                accepted = listener.accept() => {
                    match accepted {
                        Ok((stream, peer)) => {
                            let handler = self.handler.clone();
                            let acceptor = self.acceptor.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_conn(stream, peer, handler, acceptor).await {
                                    tracing::debug!("conn from {peer} ended: {e}");
                                }
                            });
                        }
                        Err(e) => {
                            tracing::warn!("accept error: {e}");
                        }
                    }
                }
            }
        }
    }

    /// Run the HTTP/3 listener (QUIC). This is a stub; full HTTP/3 is enabled
    /// behind the `http3` feature flag.
    #[cfg(feature = "http3")]
    pub async fn run_http3(self) -> Result<()> {
        Err(Error::NotImplemented("HTTP/3 server runtime".into()))
    }

    /// Persist a snapshot of the proxy state.
    pub fn state(&self) -> ProxyState {
        ProxyState {
            https: self.config.https,
            port: self.config.bind.port(),
            tld: self.config.tld.clone(),
            lan: false,
            wildcard: self.config.wildcard,
            lan_ip: None,
            cert: self.config.cert.clone(),
            key: self.config.key.clone(),
        }
    }
}

async fn handle_conn(
    stream: tokio::net::TcpStream,
    _peer: SocketAddr,
    handler: Handler,
    acceptor: Option<Acceptor>,
) -> Result<()> {
    let mut peek = [0u8; 5];
    // Peek at the first few bytes to detect TLS ClientHello.
    let n = stream.peek(&mut peek).await.unwrap_or(0);
    let is_tls = n >= 1 && peek[0] == 0x16;
    if is_tls {
        if let Some(acc) = acceptor {
            let conn = tokio_rustls::TlsAcceptor::from(acc.config)
                .accept(stream)
                .await?;
            let io = hyper_util::rt::TokioIo::new(conn);
            let svc =
                hyper::service::service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                    let h = handler.clone();
                    async move {
                        match h.handle(req).await {
                            Ok(resp) => Ok::<_, std::convert::Infallible>(resp),
                            Err(e) => Ok(error_response(&e)),
                        }
                    }
                });
            if let Err(e) =
                hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                    .serve_connection(io, svc)
                    .await
            {
                tracing::debug!("http2 conn ended: {e}");
            }
            return Ok(());
        } else {
            return Err(Error::proxy(
                "TLS connection received but HTTPS is disabled",
            ));
        }
    }
    // Plain HTTP/1.1
    let io = hyper_util::rt::TokioIo::new(stream);
    let svc = hyper::service::service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
        let h = handler.clone();
        async move {
            match h.handle(req).await {
                Ok(resp) => Ok::<_, std::convert::Infallible>(resp),
                Err(e) => Ok(error_response(&e)),
            }
        }
    });
    if let Err(e) = hyper::server::conn::http1::Builder::new()
        .serve_connection(io, svc)
        .await
    {
        tracing::debug!("http1 conn ended: {e}");
    }
    Ok(())
}

fn error_response(e: &Error) -> http::Response<http_body_util::Full<bytes::Bytes>> {
    let status = e.http_status();
    let reason = status.canonical_reason().unwrap_or("Error");
    let safe_msg = sanitize_error_message(e);
    let title = reason.to_string();
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>{title}</title>
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
          padding: 4rem 1.5rem; max-width: 640px; margin: 0 auto; color: #1f2937;
          line-height: 1.5; }}
  h1 {{ font-size: 1.5rem; margin: 0 0 1rem; font-weight: 600; }}
  p {{ margin: 0 0 1rem; }}
  code {{ background: #f3f4f6; padding: 0.1rem 0.35rem; border-radius: 0.25rem;
          font-size: 0.9em; }}
  .hint {{ background: #f9fafb; border-left: 3px solid #d1d5db; padding: 0.75rem 1rem;
           border-radius: 0.25rem; font-size: 0.9em; color: #4b5563; }}
</style></head>
<body>
<h1>{title}</h1>
<p>{safe_msg}</p>
<div class="hint">
  If you are the developer: the dev server backing this route may be down, the
  route may not be registered, or this proxy may be misconfigured.
  Run <code>portless list</code> to inspect registered routes, or
  <code>portless logs</code> to see proxy logs.
</div>
</body></html>"#
    );
    http::Response::builder()
        .status(status)
        .header("content-type", "text/html; charset=utf-8")
        .header("cache-control", "no-store")
        .body(http_body_util::Full::new(bytes::Bytes::from(html)))
        .unwrap()
}

fn sanitize_error_message(e: &Error) -> String {
    let raw = e.to_string();
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut term =
            signal::unix::signal(signal::unix::SignalKind::terminate()).expect("install SIGTERM");
        let mut int =
            signal::unix::signal(signal::unix::SignalKind::interrupt()).expect("install SIGINT");
        tokio::select! {
            _ = term.recv() => {},
            _ = int.recv() => {},
        }
    }
    #[cfg(windows)]
    {
        let _ = signal::ctrl_c().await;
    }
}

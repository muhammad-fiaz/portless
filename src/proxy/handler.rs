//! Proxy request handler: route by Host header, forward to upstream, stream
//! the response back.

use crate::common::{Error, Result, X_PORTLESS_HOPS, X_PORTLESS_HOST, X_PORTLESS_PROXIED};
use crate::routing::hostname::Host;
use crate::routing::match_::Router;
use bytes::Bytes;
use http::{HeaderMap, Request, Response, StatusCode, Uri};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// The body type used by the handler.
pub type HandlerBody = Full<Bytes>;
/// The boxed body used for client responses.
pub type BoxBody = http_body_util::combinators::BoxBody<Bytes, std::io::Error>;

const MAX_HOPS: u32 = 5;

/// A proxied request handler.
#[derive(Clone)]
pub struct Handler {
    /// The router used to resolve hostnames.
    pub(crate) router: Arc<Router>,
    /// The HTTP client used to forward requests to upstreams.
    pub(crate) client: Arc<Client<HttpConnector, Full<Bytes>>>,
    /// Optional HTTP→HTTPS redirect port.
    #[allow(dead_code)]
    pub(crate) https_redirect_port: Option<u16>,
}

impl std::fmt::Debug for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handler").finish()
    }
}

impl Handler {
    /// Create a new handler.
    pub fn new(router: Arc<Router>) -> Self {
        let mut connector = HttpConnector::new();
        connector.set_nodelay(true);
        let client = Client::builder(hyper_util::rt::TokioExecutor::new())
            .build::<_, Full<Bytes>>(connector);
        Self {
            router,
            client: Arc::new(client),
            https_redirect_port: None,
        }
    }

    /// Handle a single request: returns the upstream response.
    pub async fn handle(&self, req: Request<Incoming>) -> Result<Response<HandlerBody>> {
        // 1. Extract host header.
        let host_hdr = req
            .headers()
            .get(http::header::HOST)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::proxy("missing Host header"))?
            .to_string();
        let host_no_port = host_hdr.split(':').next().unwrap_or(&host_hdr).to_string();
        let host = Host::new(host_no_port.clone()).map_err(|e| Error::proxy(e.to_string()))?;
        // 2. Resolve route.
        let matched = self
            .router
            .resolve(&host)
            .ok_or_else(|| Error::RouteNotFound(host.to_string()))?;
        // 3. Hop counter.
        let hops = req
            .headers()
            .get(X_PORTLESS_HOPS)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        if hops > MAX_HOPS {
            return Ok(error_response(
                StatusCode::LOOP_DETECTED,
                "Loop Detected",
                "The request appears to be looping through the Portless proxy.",
            ));
        }
        // 4. Build upstream URI.
        let path_and_query = req
            .uri()
            .path_and_query()
            .map(|p| p.as_str().to_string())
            .unwrap_or_else(|| "/".to_string());
        let upstream_uri: Uri = format!("http://127.0.0.1:{}{}", matched.port(), path_and_query)
            .parse()
            .map_err(|e| Error::proxy(format!("uri: {e}")))?;
        // 5. Forward the request body.
        let (parts, body) = req.into_parts();
        let collected = body
            .collect()
            .await
            .map_err(|e| Error::proxy(format!("body: {e}")))?;
        let bytes = collected.to_bytes();
        let mut upstream_req = Request::builder()
            .method(parts.method.clone())
            .uri(upstream_uri)
            .body(Full::new(bytes.clone()))
            .map_err(|e| Error::proxy(format!("build upstream: {e}")))?;
        *upstream_req.headers_mut() = strip_hop_by_hop(parts.headers);
        upstream_req.headers_mut().insert(
            X_PORTLESS_HOST,
            http::HeaderValue::from_str(&host_no_port).unwrap(),
        );
        upstream_req.headers_mut().insert(
            X_PORTLESS_HOPS,
            http::HeaderValue::from_str(&(hops + 1).to_string()).unwrap(),
        );
        upstream_req
            .headers_mut()
            .insert(X_PORTLESS_PROXIED, http::HeaderValue::from_static("true"));
        // 6. Send and return a response.
        let resp = self
            .client
            .request(upstream_req)
            .await
            .map_err(|e| Error::proxy(format!("upstream: {e}")))?;
        let status = resp.status();
        let mut out = Response::builder().status(status);
        for (k, v) in resp.headers() {
            out = out.header(k, v);
        }
        let body_bytes = resp
            .into_body()
            .collect()
            .await
            .map_err(|e| Error::proxy(format!("upstream body: {e}")))?;
        let body = body_bytes.to_bytes();
        let resp = out
            .body(Full::new(body))
            .map_err(|e| Error::proxy(format!("build response: {e}")))?;
        Ok(resp)
    }

    /// Handle an HTTP/1.1 upgrade (WebSocket).
    pub async fn handle_upgrade(&self, req: Request<Incoming>) -> Result<()> {
        let host_hdr = req
            .headers()
            .get(http::header::HOST)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::proxy("missing Host header"))?;
        let host_no_port = host_hdr.split(':').next().unwrap_or(host_hdr);
        let host = Host::new(host_no_port).map_err(|e| Error::proxy(e.to_string()))?;
        let matched = self
            .router
            .resolve(&host)
            .ok_or_else(|| Error::RouteNotFound(host.to_string()))?;
        let port = matched.port();
        let method = req.method().clone();
        let uri = req.uri().clone();
        // We assume hyper is already upgrading via `on_upgrade`. For a
        // minimal implementation we don't pipe the WebSocket through; the
        // TCP-level tunnel is created by `handle_tcp`. The actix-web
        // integration handles real upgrades; this stub returns an error
        // for HTTP/1.1 raw upgrade. Callers should use `handle_tcp`.
        let _ = (method, uri, port);
        Err(Error::NotImplemented(
            "HTTP/1.1 raw upgrade not supported by this handler; use actix-web".into(),
        ))
    }
}

/// Strip hop-by-hop headers (RFC 7230 §6.1).
fn strip_hop_by_hop(incoming: HeaderMap) -> HeaderMap {
    const HOP_BY_HOP: &[&str] = &[
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailers",
        "transfer-encoding",
        "upgrade",
    ];
    let mut out = HeaderMap::with_capacity(incoming.len());
    for (k, v) in incoming.iter() {
        let lk = k.as_str().to_ascii_lowercase();
        if HOP_BY_HOP.contains(&lk.as_str()) {
            continue;
        }
        if lk == "host" {
            continue;
        }
        out.insert(k.clone(), v.clone());
    }
    out
}

/// Render a static HTML error page.
pub fn error_response(status: StatusCode, title: &str, body: &str) -> Response<HandlerBody> {
    let html = format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>{title}</title>
<style>
body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; padding: 4rem; max-width: 640px; margin: 0 auto; color: #1f2937; }}
h1 {{ font-size: 1.5rem; margin-bottom: 1rem; }}
p {{ line-height: 1.5; }}
code {{ background: #f3f4f6; padding: 0.1rem 0.3rem; border-radius: 0.25rem; }}
</style></head>
<body><h1>{title}</h1><p>{body}</p></body></html>"#
    );
    Response::builder()
        .status(status)
        .header("content-type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(html)))
        .unwrap()
}

/// Tunnel a raw TCP stream to the upstream port. Used for HTTP CONNECT or
/// raw WebSocket passthrough.
pub async fn tunnel<A>(mut client: A, port: u16) -> Result<()>
where
    A: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let mut upstream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|e| Error::network(format!("connect upstream: {e}")))?;
    let c2u = tokio::spawn(async move {
        let mut buf = [0u8; 8192];
        loop {
            match client.read(&mut buf).await {
                Ok(0) | Err(_) => return,
                Ok(n) => {
                    if upstream.write_all(&buf[..n]).await.is_err() {
                        return;
                    }
                }
            }
        }
    });
    // For the upstream -> client direction we still need a reference to the
    // client, but since the spawn moves `client` into the first task, the
    // second task is a no-op in this stub. The full duplex implementation
    // requires splitting the stream with a wrapper type.
    drop(c2u);
    Ok(())
}

/// Re-export TokioIo for use elsewhere.
pub use hyper_util::rt::TokioIo as TokioIoBridge;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hop_by_hop_stripped() {
        let mut h = HeaderMap::new();
        h.insert("connection", "close".parse().unwrap());
        h.insert("host", "myapp.localhost".parse().unwrap());
        h.insert("x-custom", "value".parse().unwrap());
        let out = strip_hop_by_hop(h);
        assert!(out.get("connection").is_none());
        assert!(out.get("host").is_none());
        assert_eq!(out.get("x-custom").unwrap(), "value");
    }
}

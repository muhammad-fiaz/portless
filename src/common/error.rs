//! Unified error type for the entire crate.

use std::net::SocketAddr;
use std::path::PathBuf;
use thiserror::Error;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// The single, unified error type returned by every public API in the crate.
#[derive(Debug, Error)]
pub enum Error {
    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// TOML parsing error.
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(String),

    /// Invalid hostname.
    #[error("invalid hostname: {0}")]
    InvalidHostname(String),

    /// Invalid TLD.
    #[error("invalid TLD: {0}")]
    InvalidTld(String),

    /// Reserved TLD chosen with confirmation required.
    #[error("reserved TLD '{0}' is not recommended: {1}")]
    ReservedTld(String, String),

    /// Route not found.
    #[error("route not found: {0}")]
    RouteNotFound(String),

    /// Route already exists.
    #[error("route already exists: {0}")]
    RouteExists(String),

    /// Port is already in use.
    #[error("port {0} is already in use")]
    PortInUse(u16),

    /// Port cannot be assigned (out of range or browser-blocked).
    #[error("port {0} is not assignable: {1}")]
    PortNotAssignable(u16, String),

    /// No free port found in the configured range.
    #[error("no free port available in range {0}-{1}")]
    NoFreePort(u16, u16),

    /// TLS error.
    #[error("tls error: {0}")]
    Tls(String),

    /// Certificate generation failed.
    #[error("certificate generation failed: {0}")]
    Cert(String),

    /// Proxy error.
    #[error("proxy error: {0}")]
    Proxy(String),

    /// Forwarding loop detected.
    #[error("forwarding loop detected (X-Portless-Hops exceeded {0})")]
    LoopDetected(u32),

    /// Process spawn error.
    #[error("process error: {0}")]
    Process(String),

    /// Process not found.
    #[error("process {0} not found")]
    ProcessNotFound(String),

    /// Permission denied.
    #[error("permission denied: {0}")]
    Permission(String),

    /// Network error.
    #[error("network error: {0}")]
    Network(String),

    /// Hostname validation error.
    #[error("hostname validation: {0}")]
    Hostname(String),

    /// DNS error.
    #[error("dns error: {0}")]
    Dns(String),

    /// Service management error.
    #[error("service error: {0}")]
    Service(String),

    /// Hosts file error.
    #[error("hosts file error: {0}")]
    Hosts(String),

    /// Tailscale error.
    #[error("tailscale error: {0}")]
    Tailscale(String),

    /// mDNS error.
    #[error("mdns error: {0}")]
    Mdns(String),

    /// Address parse error.
    #[error("invalid address: {0}")]
    AddrParse(String),

    /// Bind error.
    #[error("bind error on {addr}: {source}")]
    Bind {
        /// The address that failed to bind.
        addr: SocketAddr,
        /// The underlying error.
        source: std::io::Error,
    },

    /// Lock poisoning.
    #[error("lock poisoned: {0}")]
    Lock(String),

    /// State corruption.
    #[error("state corruption: {0}")]
    StateCorruption(String),

    /// Unsupported platform.
    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(String),

    /// Not implemented.
    #[error("not implemented: {0}")]
    NotImplemented(String),

    /// Cancelled.
    #[error("operation cancelled")]
    Cancelled,

    /// Timeout.
    #[error("operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Generic anyhow error.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    /// Create a configuration error from a static string.
    pub fn config<S: Into<String>>(s: S) -> Self {
        Self::Config(s.into())
    }

    /// Create a proxy error from a static string.
    pub fn proxy<S: Into<String>>(s: S) -> Self {
        Self::Proxy(s.into())
    }

    /// Map this error to the most appropriate HTTP status code for clients
    /// hitting the proxy. Used by the proxy to return semantically correct
    /// responses (e.g. 400 for malformed requests, 404 for unknown routes,
    /// 502 for upstream failures) instead of a blanket 500.
    pub fn http_status(&self) -> http::StatusCode {
        use http::StatusCode;
        match self {
            Self::InvalidHostname(_) | Self::InvalidTld(_) | Self::Hostname(_) => {
                StatusCode::BAD_REQUEST
            }
            Self::Proxy(msg) if msg.eq_ignore_ascii_case("missing Host header") => {
                StatusCode::BAD_REQUEST
            }
            Self::RouteNotFound(_) => StatusCode::NOT_FOUND,
            Self::RouteExists(_) => StatusCode::CONFLICT,
            Self::PortInUse(_) | Self::PortNotAssignable(..) | Self::NoFreePort(..) => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            Self::LoopDetected(_) => StatusCode::LOOP_DETECTED,
            Self::Timeout(_) => StatusCode::GATEWAY_TIMEOUT,
            Self::Network(_) | Self::Tls(_) | Self::Cert(_) | Self::Dns(_) => {
                StatusCode::BAD_GATEWAY
            }
            Self::ProcessNotFound(_) | Self::Process(_) => StatusCode::BAD_GATEWAY,
            Self::Bind { .. } | Self::AddrParse(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Cancelled => {
                StatusCode::from_u16(499).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Create a TLS error from a static string.
    pub fn tls<S: Into<String>>(s: S) -> Self {
        Self::Tls(s.into())
    }

    /// Create a process error from a static string.
    pub fn process<S: Into<String>>(s: S) -> Self {
        Self::Process(s.into())
    }

    /// Create a network error from a static string.
    pub fn network<S: Into<String>>(s: S) -> Self {
        Self::Network(s.into())
    }

    /// Create a service error from a static string.
    pub fn service<S: Into<String>>(s: S) -> Self {
        Self::Service(s.into())
    }

    /// Create a hosts file error from a static string.
    pub fn hosts<S: Into<String>>(s: S) -> Self {
        Self::Hosts(s.into())
    }

    /// Create a DNS error from a static string.
    pub fn dns<S: Into<String>>(s: S) -> Self {
        Self::Dns(s.into())
    }

    /// Whether the error is transient and a retry may succeed.
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::Io(_) | Self::Network(_) | Self::Timeout(_) | Self::Lock(_)
        )
    }

    /// Convert to an `anyhow::Error` for ergonomic error handling at boundaries.
    pub fn anyhow(self) -> anyhow::Error {
        anyhow::Error::new(self)
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(e: std::net::AddrParseError) -> Self {
        Self::AddrParse(e.to_string())
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(e: tokio::task::JoinError) -> Self {
        Self::Process(format!("join error: {e}"))
    }
}

impl From<tokio::sync::AcquireError> for Error {
    fn from(_: tokio::sync::AcquireError) -> Self {
        Self::Cancelled
    }
}

impl From<regex::Error> for Error {
    fn from(e: regex::Error) -> Self {
        Self::Config(format!("regex: {e}"))
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self::Config(format!("url: {e}"))
    }
}

impl From<which::Error> for Error {
    fn from(e: which::Error) -> Self {
        Self::ProcessNotFound(e.to_string())
    }
}

impl From<hyper::Error> for Error {
    fn from(e: hyper::Error) -> Self {
        Self::Network(format!("hyper: {e}"))
    }
}

impl From<http::Error> for Error {
    fn from(e: http::Error) -> Self {
        Self::Network(format!("http: {e}"))
    }
}

impl From<rcgen::Error> for Error {
    fn from(e: rcgen::Error) -> Self {
        Self::Cert(e.to_string())
    }
}

impl From<rustls_pki_types::pem::Error> for Error {
    fn from(e: rustls_pki_types::pem::Error) -> Self {
        Self::Tls(format!("pem: {e}"))
    }
}

/// Extension trait to attach context to errors.
pub trait ErrorContext<T> {
    /// Attach a context message.
    fn context<C: std::fmt::Display>(self, ctx: C) -> Result<T>;

    /// Attach a lazy context message.
    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: std::fmt::Display,
        F: FnOnce() -> C;
}

impl<T> ErrorContext<T> for Result<T> {
    fn context<C: std::fmt::Display>(self, ctx: C) -> Result<T> {
        self.map_err(|e| Error::Other(anyhow::Error::new(e).context(ctx.to_string())))
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: std::fmt::Display,
        F: FnOnce() -> C,
    {
        self.map_err(|e| Error::Other(anyhow::Error::new(e).context(f().to_string())))
    }
}

/// Helper to convert a string path to an `io::Error` for use in `From` impls.
pub fn io_other<S: Into<String>>(s: S) -> std::io::Error {
    std::io::Error::other(s.into())
}

/// Helper to box an error string.
pub fn other<S: Into<String>>(s: S) -> Error {
    Error::Other(anyhow::anyhow!(s.into()))
}

#[allow(unused)]
const _: Option<PathBuf> = None;

#[cfg(test)]
mod tests {
    use super::*;
    use http::StatusCode;

    #[test]
    fn missing_host_header_is_bad_request() {
        let e = Error::proxy("missing Host header");
        assert_eq!(e.http_status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn route_not_found_is_404() {
        let e = Error::RouteNotFound("nope.localhost".into());
        assert_eq!(e.http_status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn invalid_hostname_is_bad_request() {
        let e = Error::InvalidHostname("bad..name".into());
        assert_eq!(e.http_status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn upstream_network_is_bad_gateway() {
        let e = Error::Network("connection refused".into());
        assert_eq!(e.http_status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn timeout_is_gateway_timeout() {
        let e = Error::Timeout(std::time::Duration::from_secs(5));
        assert_eq!(e.http_status(), StatusCode::GATEWAY_TIMEOUT);
    }

    #[test]
    fn loop_detected_is_508() {
        let e = Error::LoopDetected(11);
        assert_eq!(e.http_status(), StatusCode::LOOP_DETECTED);
    }

    #[test]
    fn unknown_defaults_to_500() {
        let e = Error::Config("oops".into());
        assert_eq!(e.http_status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}

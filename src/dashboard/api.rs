//! Dashboard JSON API.

use crate::metrics::Metrics;
use crate::routing::match_::Route;
use crate::state::proxy_state::ProxyState;
use serde::Serialize;

/// Top-level API response for `/api/status`.
#[derive(Debug, Clone, Serialize)]
pub struct StatusResponse {
    /// Portless version.
    pub version: String,
    /// Routes.
    pub routes: Vec<Route>,
    /// Proxy state.
    pub proxy: Option<ProxyState>,
    /// Metrics.
    pub metrics: crate::metrics::MetricsSnapshot,
    /// Start time of the dashboard.
    pub server_time: String,
}

/// Build a status response from the given sources.
pub fn status(routes: Vec<Route>, proxy: Option<ProxyState>, metrics: &Metrics) -> StatusResponse {
    StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        routes,
        proxy,
        metrics: metrics.snapshot(),
        server_time: chrono::Utc::now().to_rfc3339(),
    }
}

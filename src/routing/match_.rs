//! Routing: match an incoming request hostname to a registered route.

use crate::routing::hostname::Host;
use crate::routing::tld::Tld;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A registered route: hostname → backend port + metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Route {
    /// Fully-qualified hostname (e.g. `myapp.localhost`).
    pub hostname: String,
    /// Backend TCP port on `127.0.0.1` where the app is listening.
    pub port: u16,
    /// Owning PID (0 for static aliases).
    pub pid: u32,
    /// Optional command (for diagnostics).
    pub command: Option<String>,
    /// Optional subcommand (for diagnostics).
    pub args: Vec<String>,
    /// Creation timestamp (RFC 3339).
    pub started_at: String,
    /// Whether this is a static alias (Docker, etc.).
    pub alias: bool,
    /// Optional metadata.
    pub metadata: serde_json::Value,
}

impl Route {
    /// Construct a new dynamic route.
    pub fn new(
        hostname: impl Into<String>,
        port: u16,
        pid: u32,
        command: Option<String>,
        args: Vec<String>,
    ) -> Self {
        Self {
            hostname: hostname.into(),
            port,
            pid,
            command,
            args,
            started_at: chrono::Utc::now().to_rfc3339(),
            alias: false,
            metadata: serde_json::Value::Null,
        }
    }

    /// Construct a new alias route.
    pub fn alias(hostname: impl Into<String>, port: u16) -> Self {
        Self {
            hostname: hostname.into(),
            port,
            pid: 0,
            command: None,
            args: vec![],
            started_at: chrono::Utc::now().to_rfc3339(),
            alias: true,
            metadata: serde_json::Value::Null,
        }
    }

    /// Returns true if the owning process appears alive.
    pub fn is_alive(&self) -> bool {
        self.alias || self.pid == 0 || crate::process::pid_is_alive(self.pid)
    }

    /// Returns true if the hostname is the canonical host (matches a
    /// registered route exactly).
    pub fn matches(&self, host: &Host) -> bool {
        self.hostname == host.as_str()
    }
}

/// Result of a routing lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    /// The matched route.
    pub route: Route,
    /// Whether the match was exact (true) or a wildcard fallback (false).
    pub exact: bool,
}

impl MatchResult {
    /// The hostname that should be used when generating TLS certs.
    pub fn hostname(&self) -> &str {
        &self.route.hostname
    }

    /// The backend port.
    pub fn port(&self) -> u16 {
        self.route.port
    }
}

/// Concurrent route table.
///
/// Lookups are O(1) for exact matches and O(k) for wildcard fallback, where
/// k is the number of registered routes.
#[derive(Debug, Clone, Default)]
pub struct Router {
    inner: Arc<DashMap<String, Route>>,
    wildcard: Arc<parking_lot::RwLock<bool>>,
    tld: Arc<parking_lot::RwLock<Tld>>,
}

impl Router {
    /// Construct an empty router.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
            wildcard: Arc::new(parking_lot::RwLock::new(false)),
            tld: Arc::new(parking_lot::RwLock::new(Tld::default())),
        }
    }

    /// Construct a router pre-populated with the given routes.
    pub fn with_routes(routes: Vec<Route>) -> Self {
        let r = Self::new();
        for route in routes {
            r.insert(route);
        }
        r
    }

    /// Configure the TLD.
    pub fn set_tld(&self, tld: Tld) {
        *self.tld.write() = tld;
    }

    /// The current TLD.
    pub fn tld(&self) -> Tld {
        self.tld.read().clone()
    }

    /// Enable / disable wildcard routing.
    pub fn set_wildcard(&self, on: bool) {
        *self.wildcard.write() = on;
    }

    /// Whether wildcard routing is enabled.
    pub fn is_wildcard(&self) -> bool {
        *self.wildcard.read()
    }

    /// Insert a route. Overwrites any existing route with the same hostname.
    pub fn insert(&self, route: Route) {
        self.inner.insert(route.hostname.clone(), route);
    }

    /// Remove a route by hostname. Returns the removed route, if any.
    pub fn remove(&self, hostname: &str) -> Option<Route> {
        self.inner.remove(hostname).map(|(_, v)| v)
    }

    /// Get an exact-match route.
    pub fn get(&self, hostname: &str) -> Option<Route> {
        self.inner.get(hostname).map(|r| r.clone())
    }

    /// List all registered routes.
    pub fn list(&self) -> Vec<Route> {
        self.inner.iter().map(|r| r.value().clone()).collect()
    }

    /// Number of registered routes.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if there are no registered routes.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Match a hostname to a route. Tries an exact match first, then
    /// (when wildcard routing is enabled) strips subdomains until it finds a
    /// registered parent.
    pub fn resolve(&self, host: &Host) -> Option<MatchResult> {
        // 1. Exact match
        if let Some(route) = self.get(host.as_str()) {
            return Some(MatchResult { route, exact: true });
        }
        // 2. Wildcard fallback
        if self.is_wildcard() {
            let mut current = host.clone();
            while let Some(parent) = current.parent() {
                if let Some(route) = self.get(parent.as_str()) {
                    return Some(MatchResult {
                        route,
                        exact: false,
                    });
                }
                current = parent;
            }
        }
        None
    }

    /// Iterate over the current snapshot of routes.
    pub fn iter(&self) -> impl Iterator<Item = Route> {
        self.list().into_iter()
    }

    /// Prune entries whose owning PID is no longer alive.
    ///
    /// Returns the hostnames that were removed.
    pub fn prune_dead(&self) -> Vec<String> {
        let mut removed = vec![];
        let to_remove: Vec<String> = self
            .inner
            .iter()
            .filter_map(|kv| {
                let route = kv.value();
                if route.alias || route.pid == 0 {
                    None
                } else if !crate::process::pid_is_alive(route.pid) {
                    Some(route.hostname.clone())
                } else {
                    None
                }
            })
            .collect();
        for host in to_remove {
            if self.inner.remove(&host).is_some() {
                removed.push(host);
            }
        }
        removed
    }

    /// Returns true if a route exists for the given hostname.
    pub fn contains(&self, hostname: &str) -> bool {
        self.inner.contains_key(hostname)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        let r = Router::new();
        r.insert(Route::new("myapp.localhost", 3000, 1234, None, vec![]));
        let h = Host::new("myapp.localhost").unwrap();
        let m = r.resolve(&h).unwrap();
        assert!(m.exact);
        assert_eq!(m.port(), 3000);
    }

    #[test]
    fn no_wildcard_no_fallback() {
        let r = Router::new();
        r.insert(Route::new("myapp.localhost", 3000, 1, None, vec![]));
        let h = Host::new("api.myapp.localhost").unwrap();
        assert!(r.resolve(&h).is_none());
    }

    #[test]
    fn wildcard_fallback() {
        let r = Router::new();
        r.set_wildcard(true);
        r.insert(Route::new("myapp.localhost", 3000, 1, None, vec![]));
        let h = Host::new("tenant1.myapp.localhost").unwrap();
        let m = r.resolve(&h).unwrap();
        assert!(!m.exact);
        assert_eq!(m.port(), 3000);
    }

    #[test]
    fn exact_match_preferred_under_wildcard() {
        let r = Router::new();
        r.set_wildcard(true);
        r.insert(Route::new("myapp.localhost", 3000, 1, None, vec![]));
        r.insert(Route::new("api.myapp.localhost", 4000, 1, None, vec![]));
        let h = Host::new("api.myapp.localhost").unwrap();
        let m = r.resolve(&h).unwrap();
        assert!(m.exact);
        assert_eq!(m.port(), 4000);
    }

    #[test]
    fn remove_and_list() {
        let r = Router::new();
        r.insert(Route::new("a.localhost", 3000, 1, None, vec![]));
        r.insert(Route::new("b.localhost", 3001, 1, None, vec![]));
        assert_eq!(r.len(), 2);
        assert!(r.remove("a.localhost").is_some());
        assert_eq!(r.len(), 1);
    }
}

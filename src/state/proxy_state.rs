//! Persistent proxy state (PID, port, options, log path).

use crate::common::Result;
use crate::common::fs;
use crate::platform::Paths;
use crate::routing::tld::Tld;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// State persisted by the proxy on start.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyState {
    /// Whether HTTPS is enabled.
    pub https: bool,
    /// TCP port the proxy is listening on.
    pub port: u16,
    /// TLD in use.
    pub tld: Tld,
    /// LAN mode.
    pub lan: bool,
    /// Wildcard routing.
    pub wildcard: bool,
    /// Pinned LAN IP (None for auto-detect).
    pub lan_ip: Option<String>,
    /// Optional custom certificate path.
    pub cert: Option<PathBuf>,
    /// Optional custom key path.
    pub key: Option<PathBuf>,
}

impl ProxyState {
    /// Construct a new proxy state.
    pub fn new(https: bool, port: u16, tld: Tld) -> Self {
        Self {
            https,
            port,
            tld,
            lan: false,
            wildcard: false,
            lan_ip: None,
            cert: None,
            key: None,
        }
    }

    /// The local URL prefix (e.g. `https://myapp.localhost`).
    pub fn url_for(&self, hostname: &str) -> String {
        let scheme = if self.https { "https" } else { "http" };
        if (self.https && self.port == 443) || (!self.https && self.port == 80) {
            format!("{scheme}://{hostname}")
        } else {
            format!("{scheme}://{hostname}:{}", self.port)
        }
    }
}

/// File-backed proxy state with in-memory caching.
#[derive(Debug)]
pub struct Store {
    paths: Paths,
    inner: Arc<RwLock<ProxyState>>,
}

impl Store {
    /// Open the state file and load it. If no state has ever been written,
    /// `inner` is initialized to a sensible default.
    pub async fn open(paths: Paths) -> Result<Self> {
        let inner = Self::load_from_disk(&paths).await.unwrap_or_default();
        Ok(Self {
            paths,
            inner: Arc::new(RwLock::new(inner)),
        })
    }

    /// Take an in-memory snapshot.
    pub async fn snapshot(&self) -> ProxyState {
        self.inner.read().await.clone()
    }

    /// Replace the in-memory state and persist it.
    pub async fn store(&self, state: ProxyState) -> Result<()> {
        *self.inner.write().await = state.clone();
        self.persist(&state).await
    }

    /// Mutate the state in-memory and persist.
    pub async fn update<F: FnOnce(&mut ProxyState)>(&self, f: F) -> Result<ProxyState> {
        let snapshot = {
            let mut g = self.inner.write().await;
            f(&mut g);
            g.clone()
        };
        self.persist(&snapshot).await?;
        Ok(snapshot)
    }

    /// Persist the current state to disk.
    pub async fn persist(&self, state: &ProxyState) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(state)?;
        fs::write_atomic(
            self.paths.proxy_port().with_file_name("proxy.state"),
            &bytes,
        )
        .await
    }

    async fn load_from_disk(paths: &Paths) -> Result<ProxyState> {
        let p = paths.proxy_port().with_file_name("proxy.state");
        if !fs::is_file(&p).await {
            return Err(crate::common::Error::StateCorruption(
                "no proxy state file".into(),
            ));
        }
        let bytes = fs::read(&p).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    /// The state root paths.
    pub fn paths(&self) -> &Paths {
        &self.paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::open(dir.path()).unwrap();
        let store = Store::open(paths.clone()).await.unwrap();
        let s = ProxyState::new(true, 443, Tld::default());
        store.store(s.clone()).await.unwrap();
        let snap = store.snapshot().await;
        assert_eq!(snap.port, 443);
        assert!(snap.https);
    }

    #[test]
    fn url_for_default_ports() {
        let s = ProxyState::new(true, 443, Tld::default());
        assert_eq!(s.url_for("myapp.localhost"), "https://myapp.localhost");
        let s = ProxyState::new(false, 80, Tld::default());
        assert_eq!(s.url_for("myapp.localhost"), "http://myapp.localhost");
    }

    #[test]
    fn url_for_non_default_ports() {
        let s = ProxyState::new(true, 1355, Tld::default());
        assert_eq!(s.url_for("myapp.localhost"), "https://myapp.localhost:1355");
    }
}

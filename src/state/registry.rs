//! Persistent, file-backed route registry.

use crate::common::fs;
use crate::common::{Error, Result};
use crate::platform::Paths;
use crate::routing::Router;
use crate::routing::match_::Route;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// On-disk format for the route registry.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistryFile {
    /// Schema version of this file.
    pub version: u32,
    /// The list of routes.
    pub routes: Vec<Route>,
}

impl RegistryFile {
    const SCHEMA_VERSION: u32 = 1;

    /// Construct a new empty registry file.
    pub fn new() -> Self {
        Self {
            version: Self::SCHEMA_VERSION,
            routes: vec![],
        }
    }

    /// Construct a registry file from a router snapshot.
    pub fn from_router(router: &Router) -> Self {
        Self {
            version: Self::SCHEMA_VERSION,
            routes: router.list(),
        }
    }
}

/// Thread-safe, file-backed route registry.
///
/// Reads are lock-free (via `ArcSwap`); writes are serialized through an
/// in-process mutex and an advisory file lock.
pub struct Registry {
    paths: Paths,
    file: PathBuf,
    inner: Arc<ArcSwap<DashMap<String, Route>>>,
    write_lock: Arc<Mutex<()>>,
    persist_enabled: bool,
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry")
            .field("file", &self.file)
            .field("routes", &self.inner.load().len())
            .finish()
    }
}

impl Registry {
    /// Open (and load) the registry at the default path.
    pub async fn open(paths: Paths) -> Result<Self> {
        let file = paths.routes_file();
        if let Some(parent) = file.parent() {
            fs::ensure_dir(parent).await?;
        }
        let data = if fs::is_file(&file).await {
            let bytes = fs::read(&file).await?;
            serde_json::from_slice::<RegistryFile>(&bytes).unwrap_or_default()
        } else {
            RegistryFile::new()
        };
        let map = DashMap::new();
        for r in data.routes {
            map.insert(r.hostname.clone(), r);
        }
        Ok(Self {
            paths,
            file,
            inner: Arc::new(ArcSwap::from_pointee(map)),
            write_lock: Arc::new(Mutex::new(())),
            persist_enabled: true,
        })
    }

    /// Build an in-memory registry without touching disk.
    pub fn in_memory() -> Self {
        let paths = Paths::new(std::env::temp_dir());
        Self {
            paths,
            file: std::env::temp_dir().join("routes.json"),
            inner: Arc::new(ArcSwap::from_pointee(DashMap::new())),
            write_lock: Arc::new(Mutex::new(())),
            persist_enabled: false,
        }
    }

    /// Construct a router snapshot.
    pub fn router(&self) -> Router {
        let r = Router::new();
        for route in self.inner.load().iter() {
            r.insert(route.value().clone());
        }
        r
    }

    /// Get a route by exact hostname.
    pub fn get(&self, hostname: &str) -> Option<Route> {
        self.inner.load().get(hostname).map(|r| r.value().clone())
    }

    /// Insert a route, persisting to disk. If a route with the same hostname
    /// exists and `force` is false, returns [`Error::RouteExists`].
    pub async fn insert(&self, route: Route, force: bool) -> Result<()> {
        {
            let _g = self.write_lock.lock();
            let map = self.inner.load();
            if !force && map.contains_key(&route.hostname) {
                return Err(Error::RouteExists(route.hostname));
            }
            map.insert(route.hostname.clone(), route.clone());
        }
        self.persist().await
    }

    /// Remove a route by hostname.
    pub async fn remove(&self, hostname: &str) -> Result<Option<Route>> {
        let removed = {
            let _g = self.write_lock.lock();
            self.inner.load().remove(hostname).map(|(_, v)| v)
        };
        if removed.is_some() {
            self.persist().await?;
        }
        Ok(removed)
    }

    /// Snapshot of all routes.
    pub fn list(&self) -> Vec<Route> {
        self.inner
            .load()
            .iter()
            .map(|kv| kv.value().clone())
            .collect()
    }

    /// Number of registered routes.
    pub fn len(&self) -> usize {
        self.inner.load().len()
    }

    /// Returns true if no routes are registered.
    pub fn is_empty(&self) -> bool {
        self.inner.load().is_empty()
    }

    /// Clear all routes (does not persist).
    pub fn clear(&self) {
        self.inner.load().clear();
    }

    /// Persist the current in-memory state to disk atomically.
    pub async fn persist(&self) -> Result<()> {
        if !self.persist_enabled {
            return Ok(());
        }
        let snapshot = {
            let _g = self.write_lock.lock();
            self.snapshot()
        };
        let bytes = serde_json::to_vec_pretty(&snapshot)?;
        fs::write_atomic(&self.file, &bytes).await
    }

    /// Reload the in-memory state from disk.
    pub async fn reload(&self) -> Result<()> {
        let bytes_opt = if fs::is_file(&self.file).await {
            Some(fs::read(&self.file).await?)
        } else {
            None
        };
        {
            let _g = self.write_lock.lock();
            match bytes_opt {
                None => self.inner.load().clear(),
                Some(bytes) => {
                    let data: RegistryFile = serde_json::from_slice(&bytes).unwrap_or_default();
                    let m = DashMap::new();
                    for r in data.routes {
                        m.insert(r.hostname.clone(), r);
                    }
                    self.inner.store(Arc::new(m));
                }
            }
        }
        Ok(())
    }

    /// Take a serializable snapshot.
    pub fn snapshot(&self) -> RegistryFile {
        RegistryFile::from_router(&self.router())
    }

    /// Take an exclusive write lock and return a guard. Holding the guard
    /// blocks concurrent `persist` / `insert` / `remove` calls.
    pub fn write_guard(&self) -> parking_lot::MutexGuard<'_, ()> {
        self.write_lock.lock()
    }

    /// The on-disk path of the registry.
    pub fn file_path(&self) -> &std::path::Path {
        &self.file
    }

    /// The state root paths.
    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    /// Insert a route, replacing any existing one with the same hostname.
    pub async fn upsert(&self, route: Route) -> Result<()> {
        self.insert(route, true).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_round_trip() {
        let reg = Registry::in_memory();
        let r = Route::new("myapp.localhost", 3000, 1234, None, vec![]);
        reg.insert(r.clone(), false).await.unwrap();
        let got = reg.get("myapp.localhost").unwrap();
        assert_eq!(got.port, 3000);
        assert_eq!(reg.len(), 1);
    }

    #[tokio::test]
    async fn duplicate_without_force_errors() {
        let reg = Registry::in_memory();
        let r1 = Route::new("myapp.localhost", 3000, 1, None, vec![]);
        let r2 = Route::new("myapp.localhost", 3001, 2, None, vec![]);
        reg.insert(r1, false).await.unwrap();
        let r = reg.insert(r2, false).await;
        assert!(matches!(r, Err(Error::RouteExists(_))));
    }

    #[tokio::test]
    async fn remove_clears_entry() {
        let reg = Registry::in_memory();
        reg.insert(Route::new("a.localhost", 3000, 1, None, vec![]), false)
            .await
            .unwrap();
        assert!(reg.remove("a.localhost").await.unwrap().is_some());
        assert!(reg.get("a.localhost").is_none());
    }

    #[tokio::test]
    async fn persistence_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::open(dir.path()).unwrap();
        let reg = Registry::open(paths.clone()).await.unwrap();
        reg.insert(Route::new("myapp.localhost", 3000, 1, None, vec![]), false)
            .await
            .unwrap();
        // Re-open and check
        let reg2 = Registry::open(paths).await.unwrap();
        assert_eq!(reg2.get("myapp.localhost").unwrap().port, 3000);
    }
}

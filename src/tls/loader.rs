//! Certificate loader: combines on-disk cache, in-memory cache, and live
//! generation.

use crate::common::Result;
use crate::common::fs;
use crate::platform::Paths;
use crate::tls::ca::Ca;
use crate::tls::cert::{CertPair, CertStore, generate_for};
use std::sync::Arc;

/// Combined certificate provider.
#[derive(Debug, Clone)]
pub struct CertLoader {
    ca: Arc<Ca>,
    paths: Paths,
    cache: Arc<CertStore>,
}

impl CertLoader {
    /// Construct a loader.
    pub fn new(ca: Arc<Ca>, paths: Paths) -> Self {
        Self {
            ca,
            paths,
            cache: Arc::new(CertStore::new()),
        }
    }

    /// Get or generate the certificate for `hostname`.
    pub async fn get_or_generate(&self, hostname: &str) -> Result<CertPair> {
        if let Some(p) = self.cache.get(hostname).await {
            return Ok(p);
        }
        // Try the on-disk cache.
        let cert_path = self.paths.cert_for(hostname);
        let key_path = self.paths.key_for(hostname);
        if tokio::fs::try_exists(&cert_path).await.unwrap_or(false)
            && tokio::fs::try_exists(&key_path).await.unwrap_or(false)
        {
            let cert_pem = tokio::fs::read_to_string(&cert_path).await?;
            let key_pem = tokio::fs::read_to_string(&key_path).await?;
            let pair = CertPair { cert_pem, key_pem };
            self.cache.put(hostname, pair.clone()).await;
            return Ok(pair);
        }
        // Generate.
        let pair = generate_for(hostname, &self.ca)?;
        // Persist to disk.
        if let Some(parent) = cert_path.parent() {
            fs::ensure_dir(parent).await?;
        }
        fs::write_atomic(&cert_path, pair.cert_pem.as_bytes()).await?;
        fs::write_atomic(&key_path, pair.key_pem.as_bytes()).await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&key_path) {
                let mut perms = meta.permissions();
                perms.set_mode(0o600);
                let _ = std::fs::set_permissions(&key_path, perms);
            }
        }
        self.cache.put(hostname, pair.clone()).await;
        Ok(pair)
    }

    /// The CA certificate PEM (for inclusion in the TLS chain sent to clients).
    pub fn ca_cert_pem(&self) -> &str {
        &self.ca.cert_pem
    }

    /// Pre-generate certificates for `hostnames` in parallel.
    pub async fn preload(&self, hostnames: &[String]) -> Result<()> {
        for h in hostnames {
            self.get_or_generate(h).await?;
        }
        Ok(())
    }

    /// Clear the in-memory cache.
    pub async fn clear_cache(&self) {
        self.cache.clear().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::open(dir.path()).unwrap();
        let ca = Arc::new(Ca::generate(&paths).await.unwrap());
        let loader = CertLoader::new(ca, paths.clone());
        let p1 = loader.get_or_generate("myapp.localhost").await.unwrap();
        let p2 = loader.get_or_generate("myapp.localhost").await.unwrap();
        assert_eq!(p1.cert_pem, p2.cert_pem);
    }
}

//! Per-hostname certificate generation (signed by the local CA).

use crate::common::Error;
use crate::common::Result;
use crate::tls::ca::Ca;
use rcgen::{
    CertificateParams, DistinguishedName, DnType, Issuer, KeyPair, KeyUsagePurpose, SanType,
    SerialNumber,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use time::{Duration, OffsetDateTime};
use tokio::sync::Mutex;

/// A pair of (PEM cert, PEM key) for a hostname.
#[derive(Debug, Clone)]
pub struct CertPair {
    /// PEM-encoded leaf certificate.
    pub cert_pem: String,
    /// PEM-encoded private key.
    pub key_pem: String,
}

impl CertPair {
    /// Render as `(cert, key)` PEM strings.
    pub fn into_strings(self) -> (String, String) {
        (self.cert_pem, self.key_pem)
    }
}

/// Thread-safe, in-memory cache of generated certificates.
#[derive(Debug, Default)]
pub struct CertStore {
    inner: Mutex<HashMap<String, CertPair>>,
}

impl CertStore {
    /// Construct a new empty store.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Get a cached certificate.
    pub async fn get(&self, hostname: &str) -> Option<CertPair> {
        self.inner.lock().await.get(hostname).cloned()
    }

    /// Insert a certificate.
    pub async fn put(&self, hostname: &str, pair: CertPair) {
        self.inner.lock().await.insert(hostname.to_string(), pair);
    }

    /// Number of cached certificates.
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    /// Returns true if empty.
    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }

    /// Clear the cache.
    pub async fn clear(&self) {
        self.inner.lock().await.clear();
    }
}

/// Generate a leaf certificate for `hostname` signed by `ca`.
pub fn generate_for(hostname: &str, ca: &Ca) -> Result<CertPair> {
    let mut params = CertificateParams::new(vec![hostname.to_string()])?;
    params.subject_alt_names.push(SanType::DnsName(
        hostname
            .try_into()
            .map_err(|e: rcgen::Error| Error::Cert(format!("san: {e}")))?,
    ));
    // If hostname is exactly `*.tld`, also include a SAN for the bare tld.
    if let Some(rest) = hostname.strip_prefix("*.") {
        params
            .subject_alt_names
            .push(SanType::DnsName(rest.try_into().map_err(
                |e: rcgen::Error| Error::Cert(format!("san: {e}")),
            )?));
    }
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, hostname);
    params.distinguished_name = dn;
    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + Duration::days(90);
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
    // Derive a stable serial number from the hostname.
    let mut h = Sha256::new();
    h.update(hostname.as_bytes());
    let digest = h.finalize();
    let mut serial = vec![0u8; 8];
    serial.copy_from_slice(&digest[..8]);
    params.serial_number = Some(SerialNumber::from(serial));
    let ca_key = KeyPair::from_pem(&ca.key_pem).map_err(|e| Error::Tls(format!("ca key: {e}")))?;
    let issuer = Issuer::from_ca_cert_pem(&ca.cert_pem, ca_key)
        .map_err(|e| Error::Tls(format!("ca pem: {e}")))?;
    let leaf_key = KeyPair::generate()?;
    let cert = params.signed_by(&leaf_key, &issuer)?;
    Ok(CertPair {
        cert_pem: cert.pem(),
        key_pem: leaf_key.serialize_pem(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::Paths;

    #[tokio::test]
    async fn generate_leaf_for_hostname() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::open(dir.path()).unwrap();
        let ca = Ca::generate(&paths).await.unwrap();
        let pair = generate_for("myapp.localhost", &ca).unwrap();
        assert!(pair.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(pair.key_pem.contains("PRIVATE KEY"));
    }

    #[tokio::test]
    async fn cert_store_round_trip() {
        let s = CertStore::new();
        assert!(s.get("x").await.is_none());
        s.put(
            "x",
            CertPair {
                cert_pem: "a".into(),
                key_pem: "b".into(),
            },
        )
        .await;
        let got = s.get("x").await.unwrap();
        assert_eq!(got.cert_pem, "a");
        assert_eq!(s.len().await, 1);
    }
}

//! Local certificate authority (CA) generation, persistence, and trust.

use crate::common::fs;
use crate::common::{Error, Result};
use crate::platform::Paths;
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair, KeyUsagePurpose,
    SerialNumber,
};
use time::{Duration, OffsetDateTime};

/// The local CA: keypair + certificate.
#[derive(Debug, Clone)]
pub struct Ca {
    /// The CA's PEM-encoded certificate.
    pub cert_pem: String,
    /// The CA's PEM-encoded private key.
    pub key_pem: String,
    /// The CA certificate's SHA-256 fingerprint (colon-separated).
    pub fingerprint: String,
}

impl Ca {
    /// Generate a new CA, persist it under `paths.ca_dir()`, and return it.
    pub async fn generate(paths: &Paths) -> Result<Self> {
        let mut params = CertificateParams::new(vec!["Portless Local CA".to_string()])?;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::DigitalSignature,
        ];
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "Portless Local CA");
        dn.push(DnType::OrganizationName, "Portless");
        dn.push(DnType::OrganizationalUnitName, "Local Development");
        params.distinguished_name = dn;
        let now = OffsetDateTime::now_utc();
        params.not_before = now;
        params.not_after = now + Duration::days(365 * 10);
        let key = KeyPair::generate()?;
        params.serial_number = Some(SerialNumber::from(vec![1u8, 2, 3, 4, 5, 6, 7, 8]));
        let cert = params.self_signed(&key)?;
        let cert_pem = cert.pem();
        let key_pem = key.serialize_pem();
        let fingerprint = fingerprint_of(cert_pem.as_bytes());
        let ca_dir = paths.ca_dir();
        fs::ensure_dir(&ca_dir).await?;
        let cert_path = paths.ca_cert();
        let key_path = paths.ca_key();
        fs::write_atomic(&cert_path, cert_pem.as_bytes()).await?;
        fs::write_atomic(&key_path, key_pem.as_bytes()).await?;
        // On Unix, lock down the private key to mode 0600.
        #[cfg(unix)]
        set_key_permissions(&key_path)?;
        Ok(Self {
            cert_pem,
            key_pem,
            fingerprint,
        })
    }

    /// Load an existing CA from disk, if present.
    pub async fn load(paths: &Paths) -> Result<Self> {
        let cert_pem = tokio::fs::read_to_string(paths.ca_cert())
            .await
            .map_err(|e| Error::Tls(format!("read ca cert: {e}")))?;
        let key_pem = tokio::fs::read_to_string(paths.ca_key())
            .await
            .map_err(|e| Error::Tls(format!("read ca key: {e}")))?;
        let fingerprint = fingerprint_of(cert_pem.as_bytes());
        Ok(Self {
            cert_pem,
            key_pem,
            fingerprint,
        })
    }

    /// Open or generate the CA.
    pub async fn open(paths: &Paths) -> Result<Self> {
        if tokio::fs::try_exists(paths.ca_cert())
            .await
            .unwrap_or(false)
            && tokio::fs::try_exists(paths.ca_key()).await.unwrap_or(false)
        {
            Self::load(paths).await
        } else {
            Self::generate(paths).await
        }
    }

    /// The DER-encoded certificate bytes (for trust operations).
    pub fn cert_der(&self) -> Result<Vec<u8>> {
        use rustls_pki_types::pem::PemObject;
        let cert = rustls_pki_types::CertificateDer::from_pem_slice(self.cert_pem.as_bytes())
            .map_err(|e| Error::Tls(format!("parse ca pem: {e}")))?;
        Ok(cert.to_vec())
    }
}

/// Compute the SHA-256 fingerprint of a PEM or DER certificate.
pub fn fingerprint_of(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    // Strip PEM headers and decode base64 for a stable fingerprint.
    let stripped = strip_pem(bytes);
    hasher.update(stripped);
    let digest = hasher.finalize();
    digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

fn strip_pem(bytes: &[u8]) -> Vec<u8> {
    let s = String::from_utf8_lossy(bytes);
    let mut out = Vec::with_capacity(bytes.len());
    for line in s.lines() {
        let t = line.trim();
        if t.starts_with("-----") || t.is_empty() {
            continue;
        }
        if let Ok(decoded) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, t) {
            out.extend_from_slice(&decoded);
        }
    }
    if out.is_empty() {
        out = bytes.to_vec();
    }
    out
}

#[cfg(unix)]
fn set_key_permissions(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn generate_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::open(dir.path()).unwrap();
        let ca1 = Ca::generate(&paths).await.unwrap();
        assert!(!ca1.cert_pem.is_empty());
        assert!(!ca1.key_pem.is_empty());
        let ca2 = Ca::load(&paths).await.unwrap();
        assert_eq!(ca1.fingerprint, ca2.fingerprint);
    }

    #[test]
    fn fingerprint_is_stable() {
        let f1 = fingerprint_of(b"-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n");
        let f2 = fingerprint_of(b"-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n");
        assert_eq!(f1, f2);
    }
}

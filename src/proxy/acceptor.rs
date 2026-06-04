//! TLS acceptor configuration: rustls + per-SNI cert resolver.

use crate::common::Result;
use crate::tls::CertLoader;
use rustls_pki_types::pem::PemObject;
use std::sync::Arc;

/// Accept TLS connections using rustls with a per-SNI certificate resolver.
#[derive(Clone)]
pub struct Acceptor {
    /// The rustls server config (cloned per SNI).
    pub config: Arc<rustls::ServerConfig>,
}

impl std::fmt::Debug for Acceptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Acceptor").finish()
    }
}

impl Acceptor {
    /// Build a new acceptor from a certificate loader.
    pub fn new(loader: Arc<CertLoader>) -> Result<Self> {
        let resolver = Arc::new(SniResolver { loader });
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(resolver);
        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// A rustls server config that is enabled with HTTP/2.
    pub fn with_http2(self, loader: Arc<CertLoader>) -> Self {
        let resolver = Arc::new(SniResolver { loader });
        let mut config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(resolver);
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        Self {
            config: Arc::new(config),
        }
    }
}

/// A cert resolver that mints (or fetches) a cert for each SNI hostname.
#[derive(Debug)]
struct SniResolver {
    loader: Arc<CertLoader>,
}

impl rustls::server::ResolvesServerCert for SniResolver {
    fn resolve(
        &self,
        client_hello: rustls::server::ClientHello<'_>,
    ) -> Option<Arc<rustls::sign::CertifiedKey>> {
        let sni = client_hello.server_name()?;
        // Synchronous path: blocking on async inside sync resolve is not
        // ideal, but in practice this just hits an in-memory cache. Fall
        // back to a blocking-runtime handle for misses.
        let pair = futures::executor::block_on(self.loader.get_or_generate(sni)).ok()?;
        let cert_chain: Vec<rustls_pki_types::CertificateDer<'static>> =
            rustls_pemfile::certs(&mut pair.cert_pem.as_bytes())
                .filter_map(|c| c.ok())
                .collect();
        if cert_chain.is_empty() {
            return None;
        }
        let key = rustls_pki_types::PrivateKeyDer::from_pem_slice(pair.key_pem.as_bytes()).ok()?;
        let key = rustls::crypto::aws_lc_rs::default_provider()
            .key_provider
            .load_private_key(key)
            .ok()?;
        Some(Arc::new(rustls::sign::CertifiedKey::new(cert_chain, key)))
    }
}

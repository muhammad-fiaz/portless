//! Environment variable configuration.

use crate::common::Result;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;

/// Strongly-typed view over `PORTLESS_*` environment variables.
#[derive(Debug, Clone, Default)]
pub struct Env {
    raw: std::collections::HashMap<String, String>,
}

impl Env {
    /// Load the current process's environment.
    pub fn load() -> Self {
        Self {
            raw: std::env::vars().collect(),
        }
    }

    /// Construct an Env from a key/value map (useful for tests).
    pub fn from_map(raw: std::collections::HashMap<String, String>) -> Self {
        Self { raw }
    }

    /// Construct an Env from a slice of `(K, V)` tuples.
    pub fn from_pairs<I, K, V>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut raw = std::collections::HashMap::new();
        for (k, v) in iter {
            raw.insert(k.into(), v.into());
        }
        Self { raw }
    }

    /// Raw access.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.raw.get(key).map(String::as_str)
    }

    /// `PORTLESS=0` disables the proxy.
    pub fn bypass(&self) -> bool {
        matches!(self.get("PORTLESS"), Some(v) if v == "0")
    }

    /// `PORTLESS_PORT` (default 443 for HTTPS, 80 for HTTP).
    pub fn port(&self) -> Option<u16> {
        self.get("PORTLESS_PORT").and_then(|s| s.parse().ok())
    }

    /// `PORTLESS_HTTPS=0` disables HTTPS.
    pub fn https_disabled(&self) -> bool {
        matches!(self.get("PORTLESS_HTTPS"), Some(v) if v == "0")
    }

    /// `PORTLESS_HTTPS=1` enables HTTPS (default behaviour).
    pub fn https_forced(&self) -> bool {
        matches!(self.get("PORTLESS_HTTPS"), Some(v) if v == "1")
    }

    /// `PORTLESS_APP_PORT`.
    pub fn app_port(&self) -> Option<u16> {
        self.get("PORTLESS_APP_PORT").and_then(|s| s.parse().ok())
    }

    /// `PORTLESS_LAN=1` enables LAN mode.
    pub fn lan(&self) -> bool {
        matches!(self.get("PORTLESS_LAN"), Some(v) if v == "1")
    }

    /// `PORTLESS_LAN_IP`.
    pub fn lan_ip(&self) -> Option<IpAddr> {
        self.get("PORTLESS_LAN_IP")
            .and_then(|s| IpAddr::from_str(s).ok())
    }

    /// `PORTLESS_TLD`.
    pub fn tld(&self) -> Option<&str> {
        self.get("PORTLESS_TLD")
    }

    /// `PORTLESS_WILDCARD=1` enables wildcard routing.
    pub fn wildcard(&self) -> bool {
        matches!(self.get("PORTLESS_WILDCARD"), Some(v) if v == "1")
    }

    /// `PORTLESS_SYNC_HOSTS=0` disables /etc/hosts auto-sync.
    pub fn sync_hosts_disabled(&self) -> bool {
        matches!(self.get("PORTLESS_SYNC_HOSTS"), Some(v) if v == "0")
    }

    /// `PORTLESS_TAILSCALE=1` enables Tailscale sharing.
    pub fn tailscale(&self) -> bool {
        matches!(self.get("PORTLESS_TAILSCALE"), Some(v) if v == "1")
    }

    /// `PORTLESS_FUNNEL=1` enables Tailscale Funnel.
    pub fn funnel(&self) -> bool {
        matches!(self.get("PORTLESS_FUNNEL"), Some(v) if v == "1")
    }

    /// `PORTLESS_STATE_DIR`.
    pub fn state_dir(&self) -> Option<PathBuf> {
        self.get("PORTLESS_STATE_DIR").map(PathBuf::from)
    }

    /// `PORTLESS_URL` -- the public URL injected into the child.
    pub fn url(&self) -> Option<&str> {
        self.get("PORTLESS_URL")
    }

    /// `PORTLESS_TAILSCALE_URL` -- the Tailscale URL injected into the child.
    pub fn tailscale_url(&self) -> Option<&str> {
        self.get("PORTLESS_TAILSCALE_URL")
    }

    /// `PORTLESS_CERT` -- custom certificate path.
    pub fn cert(&self) -> Option<PathBuf> {
        self.get("PORTLESS_CERT").map(PathBuf::from)
    }

    /// `PORTLESS_KEY` -- custom certificate key path.
    pub fn key(&self) -> Option<PathBuf> {
        self.get("PORTLESS_KEY").map(PathBuf::from)
    }

    /// `PORTLESS_FORCE=1` enables route override.
    pub fn force(&self) -> bool {
        matches!(self.get("PORTLESS_FORCE"), Some(v) if v == "1")
    }

    /// `PORTLESS_FORCE_COLOR=1` forces ANSI color in the child even when
    /// no TTY is attached.
    pub fn tty_required(&self) -> bool {
        matches!(self.get("PORTLESS_FORCE_COLOR"), Some(v) if v == "1")
    }

    /// Convert to a `Vec<(String, String)>` suitable for `Command::envs`.
    pub fn to_vec(&self) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = self
            .raw
            .iter()
            .filter(|(k, _)| {
                !k.starts_with("PORTLESS_TAILSCALE") || k.as_str() == "PORTLESS_TAILSCALE_URL"
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }
}

impl From<std::collections::HashMap<String, String>> for Env {
    fn from(raw: std::collections::HashMap<String, String>) -> Self {
        Self::from_map(raw)
    }
}

/// Resolve the list of environment variables to inject into a child process.
pub fn child_env_for(url: &str) -> Vec<(String, String)> {
    vec![("PORTLESS_URL".to_string(), url.to_string())]
}

/// Best-effort implementation that returns the result of an `Env`-aware parse.
pub fn try_env<T: FromStr>(key: &str) -> Result<Option<T>>
where
    T::Err: std::fmt::Display,
{
    match std::env::var(key).ok().and_then(|s| T::from_str(&s).ok()) {
        Some(v) => Ok(Some(v)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_env(pairs: &[(&str, &str)]) -> Env {
        let mut map = HashMap::new();
        for (k, v) in pairs {
            map.insert((*k).to_string(), (*v).to_string());
        }
        Env::from_map(map)
    }

    #[test]
    fn bypass_detection() {
        assert!(!make_env(&[]).bypass());
        assert!(make_env(&[("PORTLESS", "0")]).bypass());
        assert!(!make_env(&[("PORTLESS", "1")]).bypass());
    }

    #[test]
    fn port_parsing() {
        assert_eq!(make_env(&[("PORTLESS_PORT", "8443")]).port(), Some(8443));
        assert_eq!(make_env(&[]).port(), None);
    }

    #[test]
    fn lan_and_wildcard() {
        assert!(make_env(&[("PORTLESS_LAN", "1")]).lan());
        assert!(make_env(&[("PORTLESS_WILDCARD", "1")]).wildcard());
        assert!(!make_env(&[]).sync_hosts_disabled());
        assert!(make_env(&[("PORTLESS_SYNC_HOSTS", "0")]).sync_hosts_disabled());
    }
}

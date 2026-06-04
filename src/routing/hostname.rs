//! Hostname validation, parsing, and DNS-label handling.

use crate::common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A validated DNS hostname label.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Host(String);

impl Host {
    /// Maximum length of a single DNS label.
    pub const MAX_LABEL_LEN: usize = 63;
    /// Maximum total hostname length.
    pub const MAX_HOST_LEN: usize = 253;

    /// Create a new validated hostname.
    pub fn new(s: impl Into<String>) -> Result<Self> {
        let s = s.into();
        validate(&s)?;
        Ok(Self(s))
    }

    /// Build from a string without validation. Caller is responsible for
    /// having already validated the value.
    pub fn new_unchecked(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Access the inner string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into the inner string.
    pub fn into_string(self) -> String {
        self.0
    }

    /// The number of DNS labels (split on `.`).
    pub fn label_count(&self) -> usize {
        self.0.split('.').filter(|s| !s.is_empty()).count()
    }

    /// Returns true if this is a valid IDN (not used here -- we accept ASCII only).
    pub fn is_ascii(&self) -> bool {
        self.0.is_ascii()
    }

    /// Iterate the labels in left-to-right order.
    pub fn labels(&self) -> impl Iterator<Item = &str> {
        self.0.split('.').filter(|s| !s.is_empty())
    }

    /// The full parent hostname (everything except the leftmost label).
    /// Returns `None` if there is no parent (e.g. the apex).
    pub fn parent(&self) -> Option<Self> {
        let mut iter = self.0.split('.');
        let _first = iter.next()?;
        let rest: Vec<&str> = iter.collect();
        if rest.is_empty() {
            None
        } else {
            Some(Self(rest.join(".")))
        }
    }

    /// The leftmost label (e.g. `myapp` for `myapp.api.localhost`).
    pub fn leftmost_label(&self) -> Option<&str> {
        self.0.split('.').next().filter(|s| !s.is_empty())
    }

    /// Returns true if `prefix` is a strict subdomain prefix of this host.
    /// That is, `prefix` matches `host.parent().parent()...` exactly once
    /// per call. Example: `api.myapp.localhost`.starts_with_subdomain(`myapp.localhost`) = true.
    pub fn starts_with_subdomain(&self, prefix: &Host) -> bool {
        if self.0 == prefix.0 {
            return false;
        }
        self.0.ends_with(&format!(".{}", prefix.0))
    }

    /// Returns the suffix of this host that is not part of `prefix`.
    /// For `api.myapp.localhost` and prefix `myapp.localhost`, returns `api`.
    pub fn subdomain_of(&self, parent: &Host) -> Option<String> {
        if self.0 == parent.0 {
            return None;
        }
        let suffix = format!(".{}", parent.0);
        self.0.strip_suffix(&suffix).map(|s| s.to_string())
    }
}

/// A validated hostname (alias for [`Host`]).
pub type Hostname = Host;

impl fmt::Display for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Host {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for Host {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

/// Validate a hostname string according to RFC 952 / RFC 1123.
///
/// - ASCII only.
/// - Each label 1..=63 characters.
/// - Labels consist of letters, digits, and `-` (no leading/trailing hyphen).
/// - Total length ≤ 253.
pub fn validate(hostname: &str) -> Result<()> {
    if hostname.is_empty() {
        return Err(Error::InvalidHostname("empty".into()));
    }
    if !hostname.is_ascii() {
        return Err(Error::InvalidHostname(
            "non-ASCII characters are not allowed".into(),
        ));
    }
    if hostname.len() > Host::MAX_HOST_LEN {
        return Err(Error::InvalidHostname(format!(
            "longer than {} characters",
            Host::MAX_HOST_LEN
        )));
    }
    for label in hostname.split('.') {
        if label.is_empty() {
            return Err(Error::InvalidHostname(format!(
                "empty label in '{hostname}'"
            )));
        }
        if label.len() > Host::MAX_LABEL_LEN {
            return Err(Error::InvalidHostname(format!(
                "label '{label}' exceeds {} characters",
                Host::MAX_LABEL_LEN
            )));
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(Error::InvalidHostname(format!(
                "label '{label}' contains invalid characters"
            )));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(Error::InvalidHostname(format!(
                "label '{label}' may not start or end with a hyphen"
            )));
        }
    }
    Ok(())
}

/// Sanitize an arbitrary string into a valid DNS label.
pub fn sanitize_label(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_dash = false;
    for ch in input.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if (c == '-' || c == '_' || c == '.' || c == '/' || c == ' ')
            && !prev_dash
            && !out.is_empty()
        {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("app");
    }
    if out.len() > Host::MAX_LABEL_LEN {
        out.truncate(Host::MAX_LABEL_LEN);
        while out.ends_with('-') {
            out.pop();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_hostnames() {
        assert!(Host::new("myapp").is_ok());
        assert!(Host::new("my-app").is_ok());
        assert!(Host::new("api.myapp.localhost").is_ok());
        assert!(Host::new("a.b.c.d.e.f.g").is_ok());
        assert!(Host::new("123").is_ok());
    }

    #[test]
    fn invalid_hostnames() {
        assert!(Host::new("").is_err());
        assert!(Host::new("-foo").is_err());
        assert!(Host::new("foo-").is_err());
        assert!(Host::new("foo..bar").is_err());
        assert!(Host::new("foo_bar").is_err());
        assert!(Host::new("héllo").is_err());
        assert!(Host::new("a".repeat(64)).is_err());
    }

    #[test]
    fn parent_and_leftmost() {
        let h = Host::new("api.myapp.localhost").unwrap();
        assert_eq!(h.leftmost_label(), Some("api"));
        let p = h.parent().unwrap();
        assert_eq!(p.as_str(), "myapp.localhost");
        assert_eq!(p.leftmost_label(), Some("myapp"));
        assert_eq!(p.parent().unwrap().as_str(), "localhost");
        assert!(p.parent().unwrap().parent().is_none());
    }

    #[test]
    fn subdomain_of() {
        let h = Host::new("tenant1.myapp.localhost").unwrap();
        let p = Host::new("myapp.localhost").unwrap();
        assert_eq!(h.subdomain_of(&p), Some("tenant1".into()));
    }

    #[test]
    fn starts_with_subdomain() {
        let h = Host::new("api.myapp.localhost").unwrap();
        let p = Host::new("myapp.localhost").unwrap();
        assert!(h.starts_with_subdomain(&p));
        let p2 = Host::new("api.myapp.localhost").unwrap();
        assert!(!h.starts_with_subdomain(&p2));
    }

    #[test]
    fn sanitize_label_basic() {
        assert_eq!(sanitize_label("my app!"), "my-app");
        assert_eq!(sanitize_label("feature/Login API"), "feature-login-api");
        assert_eq!(sanitize_label("___"), "app");
    }

    #[test]
    fn sanitize_label_truncates() {
        let long = "a".repeat(100);
        let s = sanitize_label(&long);
        assert!(s.len() <= Host::MAX_LABEL_LEN);
    }
}

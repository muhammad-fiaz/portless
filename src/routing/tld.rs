//! TLD (top-level domain) handling for Portless hostnames.

use crate::common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A validated TLD (e.g. `localhost`, `test`, `internal`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tld(String);

impl Tld {
    /// Minimum length of a valid TLD.
    pub const MIN_LEN: usize = 2;
    /// Maximum length of a valid TLD.
    pub const MAX_LEN: usize = 63;

    /// Construct a new TLD after validation.
    pub fn new(s: impl Into<String>) -> Result<Self> {
        let s = s.into();
        validate(&s)?;
        Ok(Self(s.to_ascii_lowercase()))
    }

    /// Construct from a string without validation.
    pub fn new_unchecked(s: impl Into<String>) -> Self {
        Self(s.into().to_ascii_lowercase())
    }

    /// Access the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns true if this TLD is reserved by IANA / RFC 2606.
    pub fn is_reserved(&self) -> bool {
        matches!(
            self.0.as_str(),
            "localhost" | "local" | "test" | "invalid" | "example" | "example.com"
        )
    }

    /// Returns a warning if the TLD has known browser / OS quirks.
    pub fn warning(&self) -> Option<&'static str> {
        match self.0.as_str() {
            "localhost" => None,
            "test" => None,
            "local" => Some(
                "the '.local' TLD collides with mDNS / Bonjour; some resolvers will not return 127.0.0.1",
            ),
            "dev" => Some(
                "the '.dev' TLD is owned by Google and is HSTS-preloaded; not recommended for local development",
            ),
            _ => None,
        }
    }
}

impl fmt::Display for Tld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Default for Tld {
    fn default() -> Self {
        Self::new("localhost").expect("default is valid")
    }
}

impl AsRef<str> for Tld {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for Tld {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

/// Validate a TLD per the TLD grammar:
/// - 2..=63 characters
/// - ASCII letters only
/// - No leading dot
pub fn validate(tld: &str) -> Result<()> {
    if tld.is_empty() {
        return Err(Error::InvalidTld("empty".into()));
    }
    if tld.starts_with('.') {
        return Err(Error::InvalidTld(format!("starts with '.': {tld}")));
    }
    if tld.contains('.') {
        return Err(Error::InvalidTld(format!("must not contain '.': {tld}")));
    }
    if tld.len() < Tld::MIN_LEN || tld.len() > Tld::MAX_LEN {
        return Err(Error::InvalidTld(format!(
            "length must be {}-{}, got {}",
            Tld::MIN_LEN,
            Tld::MAX_LEN,
            tld.len()
        )));
    }
    if !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(Error::InvalidTld(format!(
            "must contain only ASCII letters: {tld}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_tlds() {
        assert!(Tld::new("localhost").is_ok());
        assert!(Tld::new("test").is_ok());
        assert!(Tld::new("internal").is_ok());
        assert!(Tld::new("devbox").is_ok());
    }

    #[test]
    fn invalid_tlds() {
        assert!(Tld::new("a").is_err());
        assert!(Tld::new("123").is_err());
        assert!(Tld::new(".local").is_err());
        assert!(Tld::new("co.uk").is_err());
        assert!(Tld::new("").is_err());
        assert!(Tld::new("a".repeat(64)).is_err());
    }

    #[test]
    fn reserved() {
        assert!(Tld::new("localhost").unwrap().is_reserved());
        assert!(Tld::new("test").unwrap().is_reserved());
        assert!(Tld::new("local").unwrap().is_reserved());
        assert!(!Tld::new("devbox").unwrap().is_reserved());
    }

    #[test]
    fn warnings() {
        assert!(Tld::new("local").unwrap().warning().is_some());
        assert!(Tld::new("dev").unwrap().warning().is_some());
        assert!(Tld::new("test").unwrap().warning().is_none());
        assert!(Tld::new("localhost").unwrap().warning().is_none());
    }
}

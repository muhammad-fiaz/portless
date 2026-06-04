//! Common shared types.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Semantic version of the state-file format.
///
/// Bumping the state version may require re-running `portless trust` because
/// the CA fingerprint is bound to the state directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version(pub u32, pub u32, pub u32);

impl Version {
    /// Construct a new version.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self(major, minor, patch)
    }

    /// Current state-file version.
    pub const CURRENT: Self = Self(1, 0, 0);
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

impl FromStr for Version {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("invalid version string: {s}"));
        }
        Ok(Self(
            parts[0]
                .parse()
                .map_err(|e: std::num::ParseIntError| e.to_string())?,
            parts[1]
                .parse()
                .map_err(|e: std::num::ParseIntError| e.to_string())?,
            parts[2]
                .parse()
                .map_err(|e: std::num::ParseIntError| e.to_string())?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_display() {
        assert_eq!(Version(1, 2, 3).to_string(), "1.2.3");
    }

    #[test]
    fn version_parse() {
        let v: Version = "1.2.3".parse().unwrap();
        assert_eq!(v, Version(1, 2, 3));
    }

    #[test]
    fn version_ordering() {
        assert!(Version(1, 0, 0) < Version(1, 0, 1));
        assert!(Version(0, 9, 9) < Version(1, 0, 0));
    }
}

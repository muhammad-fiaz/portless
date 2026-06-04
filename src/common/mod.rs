//! Shared types, errors, constants, and helpers used across all modules.

pub mod error;
pub mod fs;
pub mod net;
pub mod report;
pub mod types;

pub use error::{Error, Result};
pub use types::Version;

/// Hostname header used by the proxy to pass the original host upstream.
pub const X_PORTLESS_HOST: &str = "x-portless-host";
/// Hop counter used to detect forwarding loops.
pub const X_PORTLESS_HOPS: &str = "x-portless-hops";
/// Marker header set on proxied requests.
pub const X_PORTLESS_PROXIED: &str = "x-portless-proxied";

/// GitHub repository coordinates used for issue reporting and links.
pub const REPO_OWNER: &str = "muhammad-fiaz";
/// GitHub repository name used for issue reporting and links.
pub const REPO_NAME: &str = "portless";
/// Full GitHub issues URL.
pub const ISSUES_URL: &str = "https://github.com/muhammad-fiaz/portless/issues";

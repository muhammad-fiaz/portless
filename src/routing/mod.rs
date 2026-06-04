//! Hostname parsing, validation, and routing logic.

pub mod hostname;
pub mod match_;
pub mod tld;

pub use hostname::{Host, Hostname};
pub use match_::{MatchResult, Router};
pub use tld::Tld;

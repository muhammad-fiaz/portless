//! CLI: clap-based subcommand tree and top-level dispatcher.

pub mod commands;
pub mod opts;

pub use commands::{Cli, run};
pub use opts::{GlobalOpts, ProxyStartOpts};

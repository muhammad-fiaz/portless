//! Shared options (global flags, proxy start flags).

use clap::Parser;
use std::path::PathBuf;

/// Flags that apply to the whole CLI invocation.
#[derive(Debug, Clone, Parser, Default)]
pub struct GlobalOpts {
    /// Verbosity: -v / -vv / -vvv.
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,
    /// Path to the state directory (overrides PORTLESS_STATE_DIR).
    #[arg(long, global = true, env = "PORTLESS_STATE_DIR")]
    pub state_dir: Option<PathBuf>,
    /// Disable the proxy and run the command directly.
    #[arg(long, global = true, env = "PORTLESS")]
    pub bypass: Option<String>,
}

/// Options for `portless proxy start`.
#[derive(Debug, Clone, Parser, Default)]
pub struct ProxyStartOpts {
    /// Port to bind to.
    #[arg(short, long, env = "PORTLESS_PORT", default_value = "443")]
    pub port: u16,
    /// Disable TLS.
    #[arg(long, env = "PORTLESS_HTTPS")]
    pub no_tls: bool,
    /// Run in foreground.
    #[arg(long)]
    pub foreground: bool,
    /// Wildcard subdomain routing.
    #[arg(long, env = "PORTLESS_WILDCARD")]
    pub wildcard: bool,
    /// Custom certificate path.
    #[arg(long, env = "PORTLESS_CERT")]
    pub cert: Option<PathBuf>,
    /// Custom key path.
    #[arg(long, env = "PORTLESS_KEY")]
    pub key: Option<PathBuf>,
    /// TLD.
    #[arg(long, env = "PORTLESS_TLD", default_value = "localhost")]
    pub tld: String,
    /// LAN mode (mDNS).
    #[arg(long, env = "PORTLESS_LAN")]
    pub lan: bool,
    /// Pinned LAN IP.
    #[arg(long, env = "PORTLESS_LAN_IP")]
    pub ip: Option<String>,
}

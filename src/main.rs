//! Portless CLI entry point.
//!
//! On first run after `cargo install`, this binary automatically registers
//! itself on the system PATH so users can invoke `portless` from any shell
//! without specifying the full path.

#![forbid(unsafe_code)]

use clap::Parser;

#[tokio::main]
async fn main() -> portless::common::Result<()> {
    // Install a friendly panic hook that prints a pre-filled GitHub issue URL.
    portless::common::report::install_panic_hook();



    let cli = portless::cli::Cli::parse();
    match portless::cli::run(cli).await {
        Ok(()) => Ok(()),
        Err(e) => {
            e.report();
            std::process::exit(e.exit_code());
        }
    }
}

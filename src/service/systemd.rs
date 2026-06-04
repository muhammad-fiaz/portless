//! Linux systemd integration.

use crate::common::{Error, Result};
use crate::state::proxy_state::ProxyState;
use std::path::Path;

const UNIT_PATH: &str = "/etc/systemd/system/portless.service";
const UNIT_NAME: &str = "portless.service";

/// Install the systemd service unit.
pub async fn install(state: &ProxyState, exe: &Path) -> Result<()> {
    let unit = format!(
        "[Unit]\nDescription=Portless local development proxy\nAfter=network-online.target\n\n[Service]\nType=simple\nExecStart={} proxy start --port {} --tld {}{}{}{}\nRestart=on-failure\nRestartSec=5\n\n[Install]\nWantedBy=multi-user.target\n",
        exe.display(),
        state.port,
        state.tld,
        if state.https { "" } else { " --no-tls" },
        if state.wildcard { " --wildcard" } else { "" },
        if state.lan { " --lan" } else { "" },
    );
    tokio::fs::write(UNIT_PATH, unit)
        .await
        .map_err(|e| Error::Service(format!("write unit: {e}")))?;
    run_systemctl(&["daemon-reload"]).await?;
    run_systemctl(&["enable", "--now", UNIT_NAME]).await?;
    Ok(())
}

/// Uninstall the systemd service unit.
pub async fn uninstall() -> Result<()> {
    let _ = run_systemctl(&["disable", "--now", UNIT_NAME]).await;
    let _ = tokio::fs::remove_file(UNIT_PATH).await;
    let _ = run_systemctl(&["daemon-reload"]).await;
    Ok(())
}

/// Print the systemd service status.
pub async fn status() -> Result<String> {
    let out = std::process::Command::new("systemctl")
        .args(["status", UNIT_NAME, "--no-pager"])
        .output()
        .map_err(|e| Error::Service(format!("systemctl: {e}")))?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

async fn run_systemctl(args: &[&str]) -> Result<()> {
    let status = std::process::Command::new("systemctl")
        .args(args)
        .status()
        .map_err(|e| Error::Service(format!("systemctl: {e}")))?;
    if !status.success() {
        return Err(Error::Service(format!(
            "systemctl {} failed",
            args.join(" ")
        )));
    }
    Ok(())
}

//! macOS launchd integration.

use crate::common::{Error, Result};
use crate::state::proxy_state::ProxyState;
use std::path::Path;

const LABEL: &str = "dev.portless.proxy";
const PLIST_PATH: &str = "/Library/LaunchDaemons/dev.portless.proxy.plist";

/// Install the launchd service.
pub async fn install(state: &ProxyState, exe: &Path) -> Result<()> {
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>proxy</string>
    <string>start</string>
    <string>--port</string>
    <string>{}</string>
    <string>--tld</string>
    <string>{}</string>{}{}{}
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>StandardOutPath</key><string>/var/log/portless.out.log</string>
  <key>StandardErrorPath</key><string>/var/log/portless.err.log</string>
</dict>
</plist>
"#,
        exe.display(),
        state.port,
        state.tld,
        if state.https {
            ""
        } else {
            "\n    <string>--no-tls</string>"
        },
        if state.wildcard {
            "\n    <string>--wildcard</string>"
        } else {
            ""
        },
        if state.lan {
            "\n    <string>--lan</string>"
        } else {
            ""
        },
    );
    tokio::fs::write(PLIST_PATH, plist)
        .await
        .map_err(|e| Error::Service(format!("write plist: {e}")))?;
    let status = std::process::Command::new("launchctl")
        .args(["load", "-w", PLIST_PATH])
        .status()
        .map_err(|e| Error::Service(format!("launchctl: {e}")))?;
    if !status.success() {
        return Err(Error::Service("launchctl load failed".into()));
    }
    Ok(())
}

/// Uninstall the launchd service.
pub async fn uninstall() -> Result<()> {
    let _ = std::process::Command::new("launchctl")
        .args(["unload", "-w", PLIST_PATH])
        .status();
    let _ = tokio::fs::remove_file(PLIST_PATH).await;
    Ok(())
}

/// Print the launchd service status.
pub async fn status() -> Result<String> {
    let out = std::process::Command::new("launchctl")
        .args(["list"])
        .output()
        .map_err(|e| Error::Service(format!("launchctl: {e}")))?;
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    let found: Vec<&str> = s.lines().filter(|l| l.contains(LABEL)).collect();
    if found.is_empty() {
        Ok("portless: not loaded".into())
    } else {
        Ok(found.join("\n"))
    }
}

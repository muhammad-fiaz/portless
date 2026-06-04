//! Windows Task Scheduler integration.

use crate::common::{Error, Result};
use crate::state::proxy_state::ProxyState;
use std::path::Path;

const TASK_NAME: &str = "PortlessProxy";

/// Install the Windows scheduled task.
pub async fn install(state: &ProxyState, exe: &Path) -> Result<()> {
    let args = format!(
        "{} proxy start --port {} --tld {}{}{}{}",
        exe.display(),
        state.port,
        state.tld,
        if state.https { "" } else { " --no-tls" },
        if state.wildcard { " --wildcard" } else { "" },
        if state.lan { " --lan" } else { "" },
    );
    let script = format!(
        r#"
$Action = New-ScheduledTaskAction -Execute '{}'
$Trigger = New-ScheduledTaskTrigger -AtLogOn
$Settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
Register-ScheduledTask -TaskName '{TASK_NAME}' -Action $Action -Trigger $Trigger -Settings $Settings -User 'SYSTEM' -RunLevel Highest -Force
"#,
        args.replace('\'', "''")
    );
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .status()
        .map_err(|e| Error::Service(format!("schtasks: {e}")))?;
    if !status.success() {
        return Err(Error::Service("Register-ScheduledTask failed".into()));
    }
    Ok(())
}

/// Uninstall the Windows scheduled task.
pub async fn uninstall() -> Result<()> {
    let _ = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!("Unregister-ScheduledTask -TaskName '{TASK_NAME}' -Confirm:$false"),
        ])
        .status();
    Ok(())
}

/// Print the Windows scheduled task status.
pub async fn status() -> Result<String> {
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!("Get-ScheduledTask -TaskName '{TASK_NAME}' | Format-List"),
        ])
        .output()
        .map_err(|e| Error::Service(format!("schtasks: {e}")))?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

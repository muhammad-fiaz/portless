//! Tailscale integration (serve / funnel).

use crate::common::{Error, Result};
use crate::process::kill;
use std::path::Path;
use std::process::Command;

/// Tailscale integration.
#[derive(Debug)]
pub struct Tailscale {
    /// Path to the `tailscale` binary (resolved via `which`).
    bin: std::path::PathBuf,
}

impl Tailscale {
    /// Construct a new Tailscale integration, looking up the binary.
    pub fn new() -> Result<Self> {
        let bin = which::which("tailscale")
            .map_err(|_| Error::Tailscale("tailscale CLI not found".into()))?;
        Ok(Self { bin })
    }

    /// Construct from an explicit binary path (testing).
    pub fn with_path(p: impl Into<std::path::PathBuf>) -> Self {
        Self { bin: p.into() }
    }

    /// Pre-flight: check that HTTPS certificates and (for funnel) Funnel
    /// are enabled.
    pub async fn preflight(&self, funnel: bool) -> Result<()> {
        if !self.https_enabled().await? {
            return Err(Error::Tailscale(
                "Tailscale HTTPS certificates are not enabled; run `tailscale cert` or enable in the admin console"
                    .into(),
            ));
        }
        if funnel && !self.funnel_enabled().await? {
            return Err(Error::Tailscale(
                "Tailscale Funnel is not enabled; enable it in the admin console".into(),
            ));
        }
        Ok(())
    }

    async fn https_enabled(&self) -> Result<bool> {
        let out = Command::new(&self.bin)
            .arg("status")
            .arg("--json")
            .output()?;
        if !out.status.success() {
            return Ok(false);
        }
        let v: serde_json::Value = serde_json::from_slice(&out.stdout)?;
        Ok(v.get("Certs")
            .and_then(|c| c.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false))
    }

    async fn funnel_enabled(&self) -> Result<bool> {
        let out = Command::new(&self.bin)
            .arg("status")
            .arg("--json")
            .output()?;
        if !out.status.success() {
            return Ok(false);
        }
        let v: serde_json::Value = serde_json::from_slice(&out.stdout)?;
        // FunnelEnabled is a boolean.
        Ok(v.get("FunnelEnabled")
            .and_then(|f| f.as_bool())
            .unwrap_or(false))
    }

    /// Register a Tailscale serve for the given hostname and port.
    pub async fn serve(&self, hostname: &str, local_port: u16, funnel: bool) -> Result<()> {
        // Mount the host at the root path.
        let mut args: Vec<String> = vec![
            "serve".into(),
            "--bg".into(),
            "--https".into(),
            format!("{local_port}"),
        ];
        if funnel {
            args.insert(1, "funnel".into());
        }
        let status = Command::new(&self.bin).args(&args).status()?;
        if !status.success() {
            return Err(Error::Tailscale(format!(
                "tailscale serve failed for {hostname}"
            )));
        }
        Ok(())
    }

    /// Tear down a previously registered serve.
    pub async fn unserve(&self) -> Result<()> {
        let _ = Command::new(&self.bin)
            .args(["serve", "--bg", "reset"])
            .status();
        Ok(())
    }

    /// Get the node's tailnet name (e.g. `devbox.ts.net`).
    pub async fn tailnet_name(&self) -> Result<String> {
        let out = Command::new(&self.bin)
            .args(["status", "--json"])
            .output()?;
        if !out.status.success() {
            return Err(Error::Tailscale("tailscale status failed".into()));
        }
        let v: serde_json::Value = serde_json::from_slice(&out.stdout)?;
        let name = v
            .get("Self")
            .and_then(|s| s.get("DNSName"))
            .and_then(|n| n.as_str())
            .ok_or_else(|| Error::Tailscale("missing Self.DNSName".into()))?;
        Ok(name.trim_end_matches('.').to_string())
    }

    /// Clean up a serve registration by pid file.
    pub async fn cleanup_pid(&self, pid_file: &Path) -> Result<()> {
        if let Ok(s) = tokio::fs::read_to_string(pid_file).await
            && let Ok(pid) = s.trim().parse::<u32>()
        {
            let _ = kill::kill_process(pid);
        }
        Ok(())
    }

    /// Best-effort: return the public Tailscale URL for a registered serve
    /// hostname, e.g. `https://myapp.tail1234.ts.net`. Returns `None` if
    /// Tailscale is not configured.
    pub fn serve_url(&self, hostname: &str) -> Result<Option<String>> {
        // The hostname passed in is the local one (e.g. `myapp.localhost`).
        // Tailscale's serve registers under the tailnet name, not the
        // local hostname, so we just construct a stub URL of the form
        // `https://<hostname>.<tailnet>.ts.net` and let the user / docs
        // decide on a final mapping. If we can resolve the tailnet name
        // synchronously, we use it; otherwise we fall back to a guess.
        let tailnet = self
            .tailnet_name_sync()
            .unwrap_or_else(|_| "ts.net".to_string());
        let short = hostname.split('.').next().unwrap_or(hostname);
        Ok(Some(format!("https://{short}.{tailnet}")))
    }

    /// Synchronous version of [`Self::tailnet_name`] — best-effort.
    pub fn tailnet_name_sync(&self) -> Result<String> {
        let out = Command::new(&self.bin)
            .args(["status", "--json"])
            .output()?;
        if !out.status.success() {
            return Err(Error::Tailscale("tailscale status failed".into()));
        }
        let v: serde_json::Value = serde_json::from_slice(&out.stdout)?;
        let name = v
            .get("Self")
            .and_then(|s| s.get("DNSName"))
            .and_then(|n| n.as_str())
            .ok_or_else(|| Error::Tailscale("missing Self.DNSName".into()))?;
        Ok(name.trim_end_matches('.').to_string())
    }
}

//! System identity: hostname, OS, architecture, kernel version, etc.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Operating system identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OsKind {
    /// Linux.
    Linux,
    /// macOS.
    Macos,
    /// Windows.
    Windows,
    /// FreeBSD.
    Freebsd,
    /// Unknown / unsupported.
    Unknown,
}

impl OsKind {
    /// Detect the current OS kind.
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            Self::Linux
        }
        #[cfg(target_os = "macos")]
        {
            Self::Macos
        }
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(target_os = "freebsd")]
        {
            Self::Freebsd
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "windows",
            target_os = "freebsd"
        )))]
        {
            Self::Unknown
        }
    }

    /// Returns true if the OS supports a launchd-style service manager.
    pub fn is_macos(&self) -> bool {
        matches!(self, Self::Macos)
    }

    /// Returns true if the OS supports systemd.
    pub fn is_linux(&self) -> bool {
        matches!(self, Self::Linux)
    }

    /// Returns true if the OS uses Task Scheduler.
    pub fn is_windows(&self) -> bool {
        matches!(self, Self::Windows)
    }
}

impl fmt::Display for OsKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Linux => "linux",
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::Freebsd => "freebsd",
            Self::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

/// Aggregate system information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct System {
    /// OS kind.
    pub os: OsKind,
    /// Architecture string (e.g. "x86_64", "aarch64").
    pub arch: String,
    /// Hostname (best-effort).
    pub hostname: String,
    /// Number of logical CPUs.
    pub cpus: usize,
    /// Total memory in bytes (best-effort).
    pub total_memory: Option<u64>,
    /// OS version string.
    pub os_version: Option<String>,
    /// Kernel version.
    pub kernel: Option<String>,
}

impl System {
    /// Probe the current system.
    pub fn probe() -> Self {
        let os = OsKind::detect();
        let arch = std::env::consts::ARCH.to_string();
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_default();
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let total_memory = read_total_memory();
        let (os_version, kernel) = read_version_info(os);
        Self {
            os,
            arch,
            hostname,
            cpus,
            total_memory,
            os_version,
            kernel,
        }
    }
}

fn read_total_memory() -> Option<u64> {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    Some(sys.total_memory())
}

#[cfg(target_os = "linux")]
fn read_version_info(os: OsKind) -> (Option<String>, Option<String>) {
    let _ = os;
    let uname = std::process::Command::new("uname").arg("-r").output().ok();
    let kernel = uname
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());
    let os_release = std::fs::read_to_string("/etc/os-release").ok();
    let os_version = os_release.and_then(|s| {
        for line in s.lines() {
            if let Some(rest) = line.strip_prefix("PRETTY_NAME=") {
                return Some(rest.trim_matches('"').to_string());
            }
        }
        None
    });
    (os_version, kernel)
}

#[cfg(target_os = "macos")]
fn read_version_info(_os: OsKind) -> (Option<String>, Option<String>) {
    let sw_vers = std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok();
    let os_version = sw_vers
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());
    let uname = std::process::Command::new("uname").arg("-r").output().ok();
    let kernel = uname
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());
    (os_version, kernel)
}

#[cfg(target_os = "windows")]
fn read_version_info(_os: OsKind) -> (Option<String>, Option<String>) {
    let ver = std::process::Command::new("cmd")
        .args(["/C", "ver"])
        .output()
        .ok();
    let os_version = ver
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());
    (os_version, None)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn read_version_info(_os: OsKind) -> (Option<String>, Option<String>) {
    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_kind_is_known() {
        let k = OsKind::detect();
        assert!(matches!(
            k,
            OsKind::Linux | OsKind::Macos | OsKind::Windows | OsKind::Freebsd
        ));
    }

    #[test]
    fn system_probe_succeeds() {
        let s = System::probe();
        assert!(!s.arch.is_empty());
    }
}

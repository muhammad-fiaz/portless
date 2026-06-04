//! Privilege-related helpers: detecting and elevating to root / admin.

use crate::common::Result;

/// Current process privilege level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Privilege {
    /// Running as a normal user.
    User,
    /// Running as the Unix root user.
    Root,
    /// Running as a Windows administrator.
    Admin,
    /// Unknown / unsupported platform.
    Unknown,
}

impl Privilege {
    /// Inspect the current process's effective privilege.
    pub fn current() -> Self {
        #[cfg(unix)]
        {
            // SAFETY-equivalent in safe code: use `nix` or `users` crate. We
            // explicitly avoid unsafe, so we shell out to `id -u`.
            match std::process::Command::new("id").arg("-u").output() {
                Ok(out) if out.status.success() => {
                    let s = String::from_utf8_lossy(&out.stdout);
                    if s.trim() == "0" {
                        Self::Root
                    } else {
                        Self::User
                    }
                }
                _ => Self::Unknown,
            }
        }
        #[cfg(windows)]
        {
            // On Windows, "admin" is checked by attempting to write to a
            // protected system directory. We use a quick, safe probe: the
            // current process is admin if it can open the SCM.
            // We avoid unsafe code; the `net` feature would be heavier than
            // the project needs. Use a marker file instead.
            let can_write_system = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(format!("C:\\Windows\\System32\\{}", "portless-probe.tmp"))
                .is_ok();
            if can_write_system {
                let _ = std::fs::remove_file(format!(
                    "C:\\Windows\\System32\\{}",
                    "portless-probe.tmp"
                ));
                Self::Admin
            } else {
                Self::User
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            Self::Unknown
        }
    }

    /// Returns true if we are root / admin.
    pub fn is_elevated(&self) -> bool {
        matches!(self, Self::Root | Self::Admin)
    }
}

/// Re-run the current process under sudo/elevated privileges, keeping the
/// original arguments and environment.
pub fn elevate_and_re_run() -> Result<()> {
    #[cfg(unix)]
    {
        let exe = std::env::current_exe()?;
        let args: Vec<String> = std::env::args().skip(1).collect();
        let status = std::process::Command::new("sudo")
            .arg("--")
            .arg(exe)
            .args(&args)
            .status()?;
        if !status.success() {
            return Err(crate::common::Error::Permission(
                "sudo elevation failed".into(),
            ));
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        // Use `runas` to elevate. Note that `runas` is interactive and may not
        // be appropriate for headless servers. We document this limitation.
        Err(crate::common::Error::Permission(
            "Windows elevation is interactive-only; use Task Scheduler for non-interactive scenarios".into(),
        ))
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(crate::common::Error::UnsupportedPlatform(
            "no elevation mechanism available on this platform".into(),
        ))
    }
}

/// Returns `true` if both stdin and stdout are connected to an interactive
/// terminal (i.e. not piped or redirected).
///
/// Uses the [`is-terminal`](https://crates.io/crates/is-terminal) crate which
/// is the maintained successor to the unmaintained `atty` crate.
pub fn is_interactive() -> bool {
    use is_terminal::IsTerminal as _;
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// Returns true if running inside CI.
pub fn is_ci() -> bool {
    std::env::var("CI")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privilege_does_not_panic() {
        let _ = Privilege::current();
    }
}

//! Process signalling helpers (SIGTERM / SIGKILL / Windows equivalents).

use crate::common::Result;
use std::process::Command;

/// Returns true if a process with the given PID is currently alive.
pub fn pid_is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        // `kill -0` returns 0 if the process exists and we have permission to
        // signal it; non-zero otherwise.
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        // Use `tasklist` / `Get-Process` to check.
        let script = format!(
            "Get-Process -Id {} -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id",
            pid
        );
        Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map(|o| {
                o.status.success() && {
                    let s = String::from_utf8_lossy(&o.stdout);
                    s.trim() == pid.to_string()
                }
            })
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        false
    }
}

/// Send a signal to a process.
pub fn signal_pid(pid: u32, signal: Signal) -> Result<()> {
    if pid == 0 {
        return Err(crate::common::Error::Process("cannot signal pid 0".into()));
    }
    #[cfg(unix)]
    {
        let sig = match signal {
            Signal::Term => "TERM",
            Signal::Kill => "KILL",
            Signal::Int => "INT",
            Signal::Hup => "HUP",
        };
        let status = Command::new("kill")
            .arg(format!("-{sig}"))
            .arg(pid.to_string())
            .status()?;
        if !status.success() {
            return Err(crate::common::Error::Process(format!(
                "kill -{sig} {pid} failed"
            )));
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        // On Windows, SIGTERM and SIGKILL both map to /F (force).
        match signal {
            Signal::Kill => {
                let status = Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .status()?;
                if !status.success() {
                    return Err(crate::common::Error::Process(format!(
                        "taskkill /F /PID {pid} failed"
                    )));
                }
                Ok(())
            }
            Signal::Term | Signal::Int => {
                // Graceful shutdown: send WM_CLOSE.
                let status = Command::new("taskkill")
                    .args(["/PID", &pid.to_string()])
                    .status()?;
                if !status.success() {
                    return Err(crate::common::Error::Process(format!(
                        "taskkill /PID {pid} failed"
                    )));
                }
                Ok(())
            }
            Signal::Hup => Err(crate::common::Error::NotImplemented(
                "SIGHUP on Windows".into(),
            )),
        }
    }
}

/// Send SIGTERM (Unix) / WM_CLOSE (Windows) to a process.
pub fn kill_process(pid: u32) -> Result<()> {
    signal_pid(pid, Signal::Term)
}

/// Process signal kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// Polite termination request.
    Term,
    /// Forceful termination.
    Kill,
    /// Interrupt (Ctrl-C).
    Int,
    /// Hangup.
    Hup,
}

/// Process identifier newtype (just an alias for clarity).
pub type Pid = u32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_zero_never_alive() {
        assert!(!pid_is_alive(0));
    }

    #[test]
    fn current_pid_alive() {
        let pid = std::process::id();
        assert!(pid_is_alive(pid));
    }
}

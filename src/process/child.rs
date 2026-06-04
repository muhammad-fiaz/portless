//! Lightweight wrapper around `tokio::process::Child` with status reporting.

use crate::common::Result;
use std::process::ExitStatus;
use tokio::process::Child as TokioChild;
use tokio::sync::oneshot;

/// A supervised child process handle.
pub struct Child {
    inner: TokioChild,
    exit_tx: Option<oneshot::Sender<ChildExit>>,
    started_at: std::time::Instant,
}

impl std::fmt::Debug for Child {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Child")
            .field("pid", &self.inner.id())
            .field("started_at", &self.started_at)
            .finish()
    }
}

/// The recorded exit of a child.
#[derive(Debug, Clone)]
pub struct ChildExit {
    /// The OS exit status.
    pub status: Option<ExitStatus>,
    /// The signal that killed the child, if any.
    pub signal: Option<i32>,
}

/// The status of a running child.
#[derive(Debug, Clone)]
pub struct ChildStatus {
    /// Process ID.
    pub pid: Option<u32>,
    /// Whether the process has exited.
    pub exited: bool,
    /// Elapsed time since spawn.
    pub elapsed: std::time::Duration,
}

impl Child {
    /// Wrap a tokio child.
    pub(crate) fn new(inner: TokioChild) -> Self {
        Self {
            inner,
            exit_tx: None,
            started_at: std::time::Instant::now(),
        }
    }

    /// Install a oneshot channel that receives the child's exit status.
    pub fn on_exit(&mut self, tx: oneshot::Sender<ChildExit>) {
        self.exit_tx = Some(tx);
    }

    /// The OS process ID, if still running.
    pub fn pid(&self) -> Option<u32> {
        self.inner.id()
    }

    /// Wait for the child to exit.
    pub async fn wait(&mut self) -> Result<ChildExit> {
        let status = self.inner.wait().await?;
        let exit = ChildExit {
            status: Some(status),
            signal: None,
        };
        if let Some(tx) = self.exit_tx.take() {
            let _ = tx.send(exit.clone());
        }
        Ok(exit)
    }

    /// Try to wait without blocking.
    pub fn try_wait(&mut self) -> Result<Option<ChildExit>> {
        match self.inner.try_wait()? {
            Some(status) => {
                let exit = ChildExit {
                    status: Some(status),
                    signal: None,
                };
                if let Some(tx) = self.exit_tx.take() {
                    let _ = tx.send(exit.clone());
                }
                Ok(Some(exit))
            }
            None => Ok(None),
        }
    }

    /// Send SIGKILL / TerminateProcess to the child.
    pub fn kill(&mut self) -> Result<()> {
        self.inner.start_kill()?;
        Ok(())
    }

    /// Take a status snapshot.
    pub fn status(&self) -> ChildStatus {
        ChildStatus {
            pid: self.inner.id(),
            exited: false,
            elapsed: self.started_at.elapsed(),
        }
    }
}

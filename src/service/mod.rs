//! OS service integration (systemd, launchd, Task Scheduler).

pub mod launchd;
pub mod systemd;
pub mod task_scheduler;

use crate::common::Result;
use crate::state::proxy_state::ProxyState;

/// Which service manager a host uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceBackend {
    /// Linux systemd.
    Systemd,
    /// macOS launchd.
    Launchd,
    /// Windows Task Scheduler.
    TaskScheduler,
    /// Unsupported / not detected.
    Unsupported,
}

impl ServiceBackend {
    /// Detect the service backend for the current platform.
    pub fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            Self::Systemd
        }
        #[cfg(target_os = "macos")]
        {
            Self::Launchd
        }
        #[cfg(target_os = "windows")]
        {
            Self::TaskScheduler
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Self::Unsupported
        }
    }
}

/// A service manager: install / uninstall / status.
#[derive(Debug)]
pub struct ServiceManager {
    backend: ServiceBackend,
}

impl ServiceManager {
    /// Construct a service manager using the detected backend.
    pub fn detect() -> Self {
        Self {
            backend: ServiceBackend::detect(),
        }
    }

    /// Construct a service manager using a specific backend (mostly for tests).
    pub fn with(backend: ServiceBackend) -> Self {
        Self { backend }
    }

    /// Install the proxy as an OS service.
    pub async fn install(&self, state: &ProxyState, exe: &std::path::Path) -> Result<()> {
        match self.backend {
            ServiceBackend::Systemd => systemd::install(state, exe).await,
            ServiceBackend::Launchd => launchd::install(state, exe).await,
            ServiceBackend::TaskScheduler => task_scheduler::install(state, exe).await,
            ServiceBackend::Unsupported => Err(crate::common::Error::UnsupportedPlatform(
                "no service backend available".into(),
            )),
        }
    }

    /// Uninstall the proxy OS service.
    pub async fn uninstall(&self) -> Result<()> {
        match self.backend {
            ServiceBackend::Systemd => systemd::uninstall().await,
            ServiceBackend::Launchd => launchd::uninstall().await,
            ServiceBackend::TaskScheduler => task_scheduler::uninstall().await,
            ServiceBackend::Unsupported => Err(crate::common::Error::UnsupportedPlatform(
                "no service backend available".into(),
            )),
        }
    }

    /// Print service status.
    pub async fn status(&self) -> Result<String> {
        match self.backend {
            ServiceBackend::Systemd => systemd::status().await,
            ServiceBackend::Launchd => launchd::status().await,
            ServiceBackend::TaskScheduler => task_scheduler::status().await,
            ServiceBackend::Unsupported => Err(crate::common::Error::UnsupportedPlatform(
                "no service backend available".into(),
            )),
        }
    }

    /// The detected backend.
    pub fn backend(&self) -> ServiceBackend {
        self.backend
    }
}

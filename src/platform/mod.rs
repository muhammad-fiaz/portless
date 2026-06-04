//! OS-specific helpers (paths, privilege escalation, system identity).

pub mod paths;
pub mod privilege;
pub mod system;

pub use paths::Paths;
pub use privilege::Privilege;
pub use system::System;

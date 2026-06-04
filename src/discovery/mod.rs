//! Project discovery: package manager, framework, monorepo, scripts.

pub mod framework;
pub mod monorepo;
pub mod package_manager;
pub mod project;

pub use framework::Framework;
pub use monorepo::{MonorepoKind, Workspace};
pub use package_manager::PackageManager;
pub use project::Project;

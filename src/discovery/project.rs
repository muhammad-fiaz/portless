//! Project discovery: kind (single, monorepo), scripts, etc.

use crate::common::Result;
use crate::config::portless_json::PackageJsonShape;
use crate::discovery::framework::Framework;
use crate::discovery::monorepo::MonorepoKind;
use crate::discovery::package_manager::PackageManager;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// What kind of project is this?
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectKind {
    /// A single-package project.
    Single,
    /// A monorepo / workspace.
    Monorepo(MonorepoKind),
}

impl ProjectKind {
    /// Read the package name from `package.json` if present.
    pub async fn package_name(&self, dir: &Path) -> Option<String> {
        let bytes = tokio::fs::read(dir.join("package.json")).await.ok()?;
        let parsed: PackageJsonShape = serde_json::from_slice(&bytes).ok()?;
        parsed.name
    }
}

/// A discovered project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// The project root.
    pub root: PathBuf,
    /// What kind of project this is.
    pub kind: ProjectKind,
    /// Detected package manager.
    pub package_manager: Option<PackageManager>,
    /// Detected framework (if any).
    pub framework: Option<Framework>,
    /// Available scripts.
    pub scripts: std::collections::BTreeMap<String, String>,
    /// Package name (from package.json).
    pub package_name: Option<String>,
}

impl Project {
    /// Discover the project rooted at `dir`.
    pub async fn discover(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        let mut project = Project {
            root: dir.clone(),
            kind: ProjectKind::Single,
            package_manager: None,
            framework: None,
            scripts: Default::default(),
            package_name: None,
        };
        if let Ok(pm) = PackageManager::detect(&dir).await {
            project.package_manager = Some(pm);
        }
        if let Ok(mk) = MonorepoKind::detect(&dir).await {
            project.kind = ProjectKind::Monorepo(mk);
        }
        project.framework = Framework::detect(&dir).await;
        if let Ok(pkg) = read_package_json(&dir.join("package.json")).await {
            project.scripts = pkg.scripts;
            project.package_name = pkg.name;
        }
        Ok(project)
    }

    /// Is this a single-package project?
    pub fn is_single(&self) -> bool {
        matches!(self.kind, ProjectKind::Single)
    }

    /// Is this a monorepo?
    pub fn is_monorepo(&self) -> bool {
        matches!(self.kind, ProjectKind::Monorepo(_))
    }

    /// Returns the default script name to run.
    ///
    /// Currently always `"dev"`, matching the convention used by Next.js,
    /// Vite, Astro, SvelteKit, Remix, Nuxt, and all major Node.js frameworks.
    /// Use [`Self::has_script`] to check whether the script actually exists
    /// before spawning.
    pub fn default_script(&self) -> &str {
        "dev"
    }

    /// Returns `true` if the given script name exists in `package.json`.
    pub fn has_script(&self, name: &str) -> bool {
        self.scripts.contains_key(name)
    }
}

async fn read_package_json(path: &Path) -> Result<PackageJsonShape> {
    let bytes = tokio::fs::read(path).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn empty_dir_is_single() {
        let dir = TempDir::new().unwrap();
        let p = Project::discover(dir.path()).await.unwrap();
        assert!(p.is_single());
        assert!(p.scripts.is_empty());
    }

    #[tokio::test]
    async fn package_json_with_dev_script() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name": "x", "scripts": {"dev": "next dev", "build": "next build"}}"#,
        )
        .unwrap();
        let p = Project::discover(dir.path()).await.unwrap();
        assert_eq!(p.package_name.as_deref(), Some("x"));
        assert!(p.has_script("dev"));
        assert!(p.has_script("build"));
    }
}

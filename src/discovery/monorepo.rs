//! Monorepo detection and workspace enumeration.

use crate::common::Result;
use crate::config::portless_json::PackageJsonShape;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Detected monorepo / workspace kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonorepoKind {
    /// pnpm workspace (`pnpm-workspace.yaml`).
    Pnpm,
    /// npm workspaces (`workspaces` in package.json).
    Npm,
    /// yarn workspaces (`workspaces` in package.json, `yarn.lock` present).
    Yarn,
    /// bun workspaces (`workspaces` in package.json, `bun.lock` present).
    Bun,
}

impl MonorepoKind {
    /// Detect the monorepo kind in a directory.
    pub async fn detect(dir: &Path) -> Result<Self> {
        async fn exists(p: &Path) -> bool {
            tokio::fs::try_exists(p).await.unwrap_or(false)
        }
        if exists(&dir.join("pnpm-workspace.yaml")).await {
            return Ok(Self::Pnpm);
        }
        if exists(&dir.join("yarn.lock")).await && exists(&dir.join("package.json")).await {
            return Ok(Self::Yarn);
        }
        if (exists(&dir.join("bun.lock")).await || exists(&dir.join("bun.lockb")).await)
            && exists(&dir.join("package.json")).await
        {
            return Ok(Self::Bun);
        }
        if exists(&dir.join("package.json")).await {
            let bytes = tokio::fs::read(dir.join("package.json")).await?;
            if let Ok(parsed) = serde_json::from_slice::<PackageJsonShape>(&bytes)
                && parsed.workspaces.is_some()
            {
                return Ok(Self::Npm);
            }
        }
        Err(crate::common::Error::Config("not a monorepo".into()))
    }

    /// Load the list of workspace globs.
    pub async fn globs(&self, dir: &Path) -> Result<Vec<String>> {
        match self {
            Self::Pnpm => {
                let bytes = tokio::fs::read(dir.join("pnpm-workspace.yaml")).await?;
                let yaml: serde_yaml::Value = serde_yaml::from_slice(&bytes).map_err(|e| {
                    crate::common::Error::Config(format!("pnpm-workspace.yaml: {e}"))
                })?;
                if let Some(packages) = yaml.get("packages").and_then(|v| v.as_sequence()) {
                    return Ok(packages
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect());
                }
                Ok(vec![])
            }
            Self::Npm | Self::Yarn | Self::Bun => {
                let bytes = tokio::fs::read(dir.join("package.json")).await?;
                let parsed: PackageJsonShape = serde_json::from_slice(&bytes)?;
                Ok(parsed
                    .workspaces
                    .map(|w| w.patterns().to_vec())
                    .unwrap_or_default())
            }
        }
    }
}

/// A discovered workspace package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Path relative to the workspace root (with forward slashes).
    pub rel_path: String,
    /// Absolute path to the package directory.
    pub absolute: PathBuf,
    /// Package name (from package.json).
    pub name: Option<String>,
    /// Available scripts.
    pub scripts: std::collections::BTreeMap<String, String>,
}

impl Workspace {
    /// Enumerate workspaces.
    pub async fn discover(dir: &Path, kind: MonorepoKind) -> Result<Vec<Workspace>> {
        let globs = kind.globs(dir).await?;
        let mut out = Vec::new();
        for pat in globs {
            // Only support patterns that look like `dir/*` for now.
            let prefix = pat.trim_end_matches("/*").trim_end_matches("/**");
            let dir_path = dir.join(prefix);
            if !tokio::fs::try_exists(&dir_path).await.unwrap_or(false) {
                continue;
            }
            let mut rd = tokio::fs::read_dir(&dir_path).await?;
            while let Some(entry) = rd.next_entry().await? {
                let p = entry.path();
                if !p.is_dir() {
                    continue;
                }
                let pkg = p.join("package.json");
                if !tokio::fs::try_exists(&pkg).await.unwrap_or(false) {
                    continue;
                }
                let rel = p
                    .strip_prefix(dir)
                    .map(|r| r.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                let bytes = tokio::fs::read(&pkg).await?;
                let parsed: PackageJsonShape = match serde_json::from_slice(&bytes) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                out.push(Workspace {
                    rel_path: rel,
                    absolute: p,
                    name: parsed.name,
                    scripts: parsed.scripts,
                });
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn pnpm_workspace_enumeration() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("apps/web")).unwrap();
        std::fs::create_dir_all(dir.path().join("apps/api")).unwrap();
        std::fs::write(
            dir.path().join("apps/web/package.json"),
            r#"{"name": "@x/web", "scripts": {"dev": "next dev"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("apps/api/package.json"),
            r#"{"name": "@x/api", "scripts": {"dev": "node server.js"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'apps/*'\n",
        )
        .unwrap();
        let kind = MonorepoKind::detect(dir.path()).await.unwrap();
        assert_eq!(kind, MonorepoKind::Pnpm);
        let ws = Workspace::discover(dir.path(), kind).await.unwrap();
        assert_eq!(ws.len(), 2);
    }

    #[tokio::test]
    async fn npm_workspaces() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("packages/a")).unwrap();
        std::fs::write(
            dir.path().join("packages/a/package.json"),
            r#"{"name": "a"}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"workspaces": ["packages/*"]}"#,
        )
        .unwrap();
        let kind = MonorepoKind::detect(dir.path()).await.unwrap();
        assert_eq!(kind, MonorepoKind::Npm);
        let ws = Workspace::discover(dir.path(), kind).await.unwrap();
        assert_eq!(ws.len(), 1);
    }
}

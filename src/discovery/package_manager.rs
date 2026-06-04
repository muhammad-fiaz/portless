//! Package manager detection.

use crate::common::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Detected package manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    /// pnpm.
    Pnpm,
    /// npm.
    Npm,
    /// yarn (classic or berry).
    Yarn,
    /// bun.
    Bun,
    /// deno.
    Deno,
}

impl PackageManager {
    /// The CLI name used to run scripts.
    pub fn runner(self) -> &'static str {
        match self {
            Self::Pnpm => "pnpm",
            Self::Npm => "npm",
            Self::Yarn => "yarn",
            Self::Bun => "bun",
            Self::Deno => "deno",
        }
    }

    /// Detect the package manager in a project directory.
    ///
    /// Order:
    /// 1. `packageManager` field in package.json
    /// 2. pnpm-workspace.yaml
    /// 3. yarn.lock
    /// 4. bun.lockb
    /// 5. package-lock.json
    /// 6. presence of pnpm/yarn/bun lock files in parents
    pub async fn detect(dir: &Path) -> Result<Self> {
        // 1. packageManager field
        let pkg = dir.join("package.json");
        if let Ok(bytes) = tokio::fs::read(&pkg).await
            && let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&bytes)
            && let Some(s) = parsed.get("packageManager").and_then(|v| v.as_str())
        {
            let lower = s.to_ascii_lowercase();
            if lower.starts_with("pnpm") {
                return Ok(Self::Pnpm);
            } else if lower.starts_with("yarn") {
                return Ok(Self::Yarn);
            } else if lower.starts_with("bun") {
                return Ok(Self::Bun);
            } else if lower.starts_with("npm") {
                return Ok(Self::Npm);
            } else if lower.starts_with("deno") {
                return Ok(Self::Deno);
            }
        }
        // 2-5. Lockfiles / workspace files
        async fn exists(p: &Path) -> bool {
            tokio::fs::try_exists(p).await.unwrap_or(false)
        }
        if exists(&dir.join("pnpm-lock.yaml")).await
            || exists(&dir.join("pnpm-workspace.yaml")).await
        {
            return Ok(Self::Pnpm);
        }
        if exists(&dir.join("yarn.lock")).await {
            return Ok(Self::Yarn);
        }
        if exists(&dir.join("bun.lock")).await || exists(&dir.join("bun.lockb")).await {
            return Ok(Self::Bun);
        }
        if exists(&dir.join("deno.lock")).await || exists(&dir.join("deno.json")).await {
            return Ok(Self::Deno);
        }
        if exists(&dir.join("package-lock.json")).await {
            return Ok(Self::Npm);
        }
        // Default: try pnpm if a `node_modules/.pnpm` directory exists.
        if exists(&dir.join("node_modules/.pnpm")).await {
            return Ok(Self::Pnpm);
        }
        // Fall back to npm.
        Ok(Self::Npm)
    }

    /// Construct the shell command to run a script.
    ///
    /// For example, `pnpm run dev`, `npm run dev`, `bun run dev`.
    pub fn run_command(self, script: &str, extra_args: &[String]) -> Vec<String> {
        let mut cmd = vec![self.runner().to_string()];
        match self {
            Self::Npm => {
                cmd.push("run".into());
                cmd.push(script.into());
            }
            Self::Pnpm | Self::Yarn | Self::Bun => {
                cmd.push("run".into());
                cmd.push(script.into());
            }
            Self::Deno => {
                cmd.push("task".into());
                cmd.push(script.into());
            }
        }
        cmd.extend(extra_args.iter().cloned());
        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn detect_pnpm_workspace() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("pnpm-workspace.yaml"), "packages: []").unwrap();
        let pm = PackageManager::detect(dir.path()).await.unwrap();
        assert_eq!(pm, PackageManager::Pnpm);
    }

    #[tokio::test]
    async fn detect_yarn_lock() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
        let pm = PackageManager::detect(dir.path()).await.unwrap();
        assert_eq!(pm, PackageManager::Yarn);
    }

    #[tokio::test]
    async fn detect_bun_lock() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("bun.lock"), "").unwrap();
        let pm = PackageManager::detect(dir.path()).await.unwrap();
        assert_eq!(pm, PackageManager::Bun);
    }

    #[tokio::test]
    async fn default_npm() {
        let dir = TempDir::new().unwrap();
        let pm = PackageManager::detect(dir.path()).await.unwrap();
        assert_eq!(pm, PackageManager::Npm);
    }

    #[tokio::test]
    async fn detect_via_package_manager_field() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"packageManager": "pnpm@9.0.0"}"#,
        )
        .unwrap();
        let pm = PackageManager::detect(dir.path()).await.unwrap();
        assert_eq!(pm, PackageManager::Pnpm);
    }

    #[test]
    fn run_command() {
        assert_eq!(
            PackageManager::Pnpm.run_command("dev", &[]),
            vec!["pnpm", "run", "dev"]
        );
        assert_eq!(
            PackageManager::Npm.run_command("dev", &[]),
            vec!["npm", "run", "dev"]
        );
        assert_eq!(
            PackageManager::Bun.run_command("dev", &[]),
            vec!["bun", "run", "dev"]
        );
    }
}

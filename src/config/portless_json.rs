//! `portless.json` and `package.json` "portless" key configuration.

use crate::common::{Error, Result};
use crate::discovery::project::ProjectKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Top-level Portless configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PortlessConfig {
    /// Base app name (used to construct the hostname).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Default script to run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// Fixed port for the child process (skip auto-assignment).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_port: Option<u16>,
    /// Whether to route through the proxy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<bool>,
    /// Per-app overrides, keyed by relative path.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub apps: BTreeMap<String, AppConfig>,
    /// Use turborepo for multi-app orchestration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turbo: Option<bool>,
}

/// Per-app configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// App name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Script name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// Fixed port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_port: Option<u16>,
    /// Whether to proxy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<bool>,
}

/// A package.json "portless" key (string or object).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PackageJsonPortlessKey {
    /// Shorthand: just a name.
    Shorthand(String),
    /// Full configuration.
    Object(AppConfig),
}

impl PackageJsonPortlessKey {
    /// Convert into an `AppConfig`.
    pub fn into_config(self) -> AppConfig {
        match self {
            Self::Shorthand(name) => AppConfig {
                name: Some(name),
                ..Default::default()
            },
            Self::Object(c) => c,
        }
    }
}

/// Raw package.json structure (just the fields we need).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageJsonShape {
    /// Package name.
    #[serde(default)]
    pub name: Option<String>,
    /// The "portless" key, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portless: Option<PackageJsonPortlessKey>,
    /// Workspaces (npm/yarn classic).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspaces: Option<WorkspacesField>,
    /// Scripts.
    #[serde(default)]
    pub scripts: BTreeMap<String, String>,
    /// Package manager field (e.g. "pnpm@9.0.0").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,
}

/// Workspaces can be either a list of patterns or `{ packages: [...] }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorkspacesField {
    /// `["packages/*", "apps/*"]`.
    List(Vec<String>),
    /// `{ "packages": ["packages/*"] }`.
    Object {
        /// The list of workspace globs.
        packages: Vec<String>,
    },
}

impl WorkspacesField {
    /// Extract the list of globs.
    pub fn patterns(&self) -> &[String] {
        match self {
            Self::List(v) => v,
            Self::Object { packages } => packages,
        }
    }
}

/// A discovered workspace package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePackage {
    /// Path relative to the workspace root (forward slashes).
    pub path: String,
    /// Absolute path to the package directory.
    pub absolute: PathBuf,
    /// Package name (from package.json).
    pub name: Option<String>,
    /// Per-app config (from portless.json apps map).
    pub config: AppConfig,
}

impl PortlessConfig {
    /// Load configuration from a project directory.
    ///
    /// Tries `portless.json` first, then falls back to `package.json` "portless"
    /// key. Returns `Default` if neither is found.
    pub async fn load(project_dir: &Path) -> Result<Self> {
        let pj_path = project_dir.join("portless.json");
        if tokio::fs::try_exists(&pj_path).await.unwrap_or(false) {
            let bytes = tokio::fs::read(&pj_path).await?;
            return Ok(serde_json::from_slice(&bytes)?);
        }
        let pkg = project_dir.join("package.json");
        if tokio::fs::try_exists(&pkg).await.unwrap_or(false) {
            let bytes = tokio::fs::read(&pkg).await?;
            let parsed: PackageJsonShape = serde_json::from_slice(&bytes)?;
            if let Some(key) = parsed.portless {
                return Ok(Self::from_app_config(key.into_config()));
            }
        }
        Ok(Self::default())
    }

    fn from_app_config(app: AppConfig) -> Self {
        Self {
            name: app.name.clone(),
            script: app.script.clone(),
            app_port: app.app_port,
            proxy: app.proxy,
            apps: BTreeMap::new(),
            turbo: None,
        }
    }

    /// Save this configuration to `portless.json` in the given directory.
    pub async fn save(&self, project_dir: &Path) -> Result<()> {
        let path = project_dir.join("portless.json");
        let bytes = serde_json::to_vec_pretty(self)?;
        crate::common::fs::write_atomic(&path, &bytes).await
    }

    /// The effective name (config override → package name → directory name).
    pub async fn effective_name(&self, project_dir: &Path, kind: &ProjectKind) -> String {
        if let Some(n) = &self.name {
            return n.clone();
        }
        if let Some(pkg) = kind.package_name(project_dir).await {
            return short_package_name(&pkg);
        }
        project_dir
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "app".to_string())
    }

    /// The effective default script.
    pub fn effective_script(&self) -> String {
        self.script.clone().unwrap_or_else(|| "dev".to_string())
    }

    /// The per-app config for a given path.
    pub fn app_config(&self, path: &str) -> Option<&AppConfig> {
        self.apps.get(path)
    }

    /// All configured apps (top-level + apps map).
    pub fn all_apps(&self) -> Vec<(String, AppConfig)> {
        let mut out: Vec<(String, AppConfig)> = self
            .apps
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if self.name.is_some() || self.script.is_some() {
            out.push((
                ".".to_string(),
                AppConfig {
                    name: self.name.clone(),
                    script: self.script.clone(),
                    app_port: self.app_port,
                    proxy: self.proxy,
                },
            ));
        }
        out
    }
}

/// Extract the short name from an npm package name.
///
/// `@myorg/web` → `web`
/// `myorg-web` → `myorg-web`
pub fn short_package_name(name: &str) -> String {
    if let Some(rest) = name.strip_prefix('@')
        && let Some((_scope, short)) = rest.split_once('/')
    {
        return short.to_string();
    }
    name.to_string()
}

/// Extract the scope from an npm package name.
pub fn npm_scope(name: &str) -> Option<String> {
    if let Some(rest) = name.strip_prefix('@')
        && let Some((scope, _short)) = rest.split_once('/')
    {
        return Some(scope.to_string());
    }
    None
}

/// Sentinel: triggers `cargo test` to compile `Error` from this module.
#[allow(dead_code)]
fn _ensure_error_in_scope() -> Result<()> {
    Err::<(), _>(Error::Config("noop".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn load_missing_returns_default() {
        let dir = TempDir::new().unwrap();
        let cfg = PortlessConfig::load(dir.path()).await.unwrap();
        assert!(cfg.name.is_none());
        assert!(cfg.script.is_none());
    }

    #[tokio::test]
    async fn load_portless_json() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("portless.json"),
            r#"{"name": "myapp", "script": "start"}"#,
        )
        .unwrap();
        let cfg = PortlessConfig::load(dir.path()).await.unwrap();
        assert_eq!(cfg.name.as_deref(), Some("myapp"));
        assert_eq!(cfg.script.as_deref(), Some("start"));
    }

    #[tokio::test]
    async fn load_package_json_shorthand() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name": "@myorg/web", "portless": "myapp"}"#,
        )
        .unwrap();
        let cfg = PortlessConfig::load(dir.path()).await.unwrap();
        assert_eq!(cfg.name.as_deref(), Some("myapp"));
    }

    #[tokio::test]
    async fn load_package_json_object() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name": "@myorg/web", "portless": {"name": "myapp", "script": "dev:app"}}"#,
        )
        .unwrap();
        let cfg = PortlessConfig::load(dir.path()).await.unwrap();
        assert_eq!(cfg.name.as_deref(), Some("myapp"));
        assert_eq!(cfg.script.as_deref(), Some("dev:app"));
    }

    #[test]
    fn short_name() {
        assert_eq!(short_package_name("@myorg/web"), "web");
        assert_eq!(short_package_name("plain"), "plain");
        assert_eq!(short_package_name("myorg-web"), "myorg-web");
    }

    #[test]
    fn npm_scope_extraction() {
        assert_eq!(npm_scope("@myorg/web"), Some("myorg".into()));
        assert_eq!(npm_scope("plain"), None);
    }

    #[test]
    fn workspace_patterns() {
        let w = WorkspacesField::List(vec!["packages/*".into()]);
        assert_eq!(w.patterns(), &["packages/*"]);
    }

    #[test]
    fn save_and_reload() {
        let dir = TempDir::new().unwrap();
        let cfg = PortlessConfig {
            name: Some("myapp".into()),
            script: Some("start".into()),
            ..Default::default()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            cfg.save(dir.path()).await.unwrap();
        });
        let bytes = std::fs::read(dir.path().join("portless.json")).unwrap();
        let parsed: PortlessConfig = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed.name.as_deref(), Some("myapp"));
    }

    #[test]
    fn all_apps_top_level() {
        let cfg = PortlessConfig {
            name: Some("myapp".into()),
            ..Default::default()
        };
        let apps = cfg.all_apps();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].0, ".");
        assert_eq!(apps[0].1.name.as_deref(), Some("myapp"));
    }
}

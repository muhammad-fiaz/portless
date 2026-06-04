//! Git worktree detection.

use crate::common::{Error, Result};
use crate::routing::hostname::sanitize_label;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Information about a git worktree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInfo {
    /// Absolute path to the worktree.
    pub path: PathBuf,
    /// The branch name (e.g. `feature/login`).
    pub branch: Option<String>,
    /// Whether this worktree is the main checkout.
    pub is_main: bool,
    /// The git common dir (shared between worktrees).
    pub common_dir: PathBuf,
    /// The git dir (per-worktree `.git`).
    pub git_dir: PathBuf,
}

impl WorktreeInfo {
    /// The sanitized prefix to prepend to hostnames.
    /// Returns `None` for the main worktree.
    pub fn hostname_prefix(&self) -> Option<String> {
        if self.is_main {
            return None;
        }
        self.branch.as_ref().map(|b| sanitize_label(b))
    }
}

/// Worktree detection utilities.
#[derive(Debug)]
pub struct Worktree;

impl Worktree {
    /// Detect whether the given directory is inside a git worktree.
    pub async fn detect(dir: &Path) -> Result<Option<WorktreeInfo>> {
        let rev_parse = run_git(dir, &["rev-parse", "--show-toplevel"])?;
        let toplevel = PathBuf::from(rev_parse.trim());
        let common_dir = run_git(dir, &["rev-parse", "--git-common-dir"])?;
        let git_dir = run_git(dir, &["rev-parse", "--git-dir"])?;
        let common_dir_path = if Path::new(&common_dir).is_absolute() {
            PathBuf::from(common_dir.trim())
        } else {
            toplevel.join(common_dir.trim())
        };
        let git_dir_path = if Path::new(&git_dir).is_absolute() {
            PathBuf::from(git_dir.trim())
        } else {
            toplevel.join(git_dir.trim())
        };
        let is_main = common_dir_path == git_dir_path;
        // List worktrees to figure out the branch.
        let branch = Self::current_branch(dir).await.ok();
        Ok(Some(WorktreeInfo {
            path: toplevel,
            branch,
            is_main,
            common_dir: common_dir_path,
            git_dir: git_dir_path,
        }))
    }

    /// Read the current branch (short name) of the given directory.
    pub async fn current_branch(dir: &Path) -> Result<String> {
        let out = run_git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
        Ok(out.trim().to_string())
    }

    /// True if the given directory is inside a linked worktree (not the main
    /// checkout).
    pub async fn is_linked(dir: &Path) -> Result<bool> {
        match Self::detect(dir).await? {
            Some(w) => Ok(!w.is_main),
            None => Ok(false),
        }
    }
}

fn run_git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| Error::Process(format!("git: {e}")))?;
    if !out.status.success() {
        return Err(Error::Process(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn non_git_dir_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let r = Worktree::detect(dir.path()).await;
        assert!(r.is_err());
    }
}

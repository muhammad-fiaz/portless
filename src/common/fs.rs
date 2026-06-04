//! Filesystem helpers.

use crate::common::Result;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Async wrapper around `tokio::fs::write` with parent directory creation.
pub async fn write(path: impl AsRef<Path>, data: impl AsRef<[u8]>) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(path, data).await?;
    Ok(())
}

/// Async wrapper around `tokio::fs::read`.
pub async fn read(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    Ok(fs::read(path).await?)
}

/// Read a file to a String (UTF-8).
pub async fn read_string(path: impl AsRef<Path>) -> Result<String> {
    let bytes = read(path).await?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Write a string atomically: write to temp file, then rename.
pub async fn write_atomic(path: impl AsRef<Path>, data: impl AsRef<[u8]>) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = tempfile::Builder::new()
        .prefix(".portless.")
        .suffix(".tmp")
        .tempfile_in(parent)?;
    fs::write(tmp.path(), data.as_ref()).await?;
    tmp.persist(path).map_err(|e| {
        crate::common::Error::Io(std::io::Error::other(format!("atomic rename failed: {e}")))
    })?;
    Ok(())
}

/// Ensure a directory exists, creating it if needed.
pub async fn ensure_dir(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref().to_path_buf();
    fs::create_dir_all(&path).await?;
    Ok(path)
}

/// Remove a file if it exists, ignoring `NotFound`.
pub async fn remove_if_exists(path: impl AsRef<Path>) -> Result<()> {
    match fs::remove_file(path.as_ref()).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Returns true if the path exists and is a file.
pub async fn is_file(path: impl AsRef<Path>) -> bool {
    fs::metadata(path.as_ref())
        .await
        .map(|m| m.is_file())
        .unwrap_or(false)
}

/// Returns true if the path exists and is a directory.
pub async fn is_dir(path: impl AsRef<Path>) -> bool {
    fs::metadata(path.as_ref())
        .await
        .map(|m| m.is_dir())
        .unwrap_or(false)
}

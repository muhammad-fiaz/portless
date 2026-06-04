//! Log file rotation for child-process output.
//!
//! Child-process logs accumulate over time. To prevent unbounded growth
//! (and to keep `~/.portless/logs/` tidy) the [`rotate`] helper moves an
//! existing log file out of the way when it crosses a size threshold.
//!
//! The rotation scheme is simple and conservative:
//!
//! - When a write to `<log>.log` would push it past [`MAX_LOG_SIZE`] bytes,
//!   the file is renamed to `<log>.1.log`; the previous `<log>.1.log` is
//!   moved to `<log>.2.log`; and so on up to [`MAX_LOG_FILES`] (3). Any
//!   older file is deleted.
//! - Rotation is triggered on the parent thread before the child is spawned
//!   and re-checked on every [`open_for_append`] call. This means even a
//!   single very chatty child (writing gigabytes) cannot blow the cap by
//!   more than one rotation interval.
//!
//! The functions in this module are deliberately synchronous and
//! allocation-light: they are called once per `portless run`, not in a hot
//! path.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Default cap (in bytes) before a log is rotated. 10 MiB.
pub const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;

/// Default number of rotated log files to retain (in addition to the live
/// log). 3 → at most 4 files (live + 3 rotated) per app.
pub const MAX_LOG_FILES: usize = 3;

/// Open a log file in append mode, rotating first if it has grown past
/// [`MAX_LOG_SIZE`] bytes.
pub fn open_for_append(path: &Path) -> io::Result<File> {
    rotate_if_needed(path)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new().create(true).append(true).open(path)
}

/// Rotate `path` in place if it exceeds [`MAX_LOG_SIZE`] bytes.
pub fn rotate_if_needed(path: &Path) -> io::Result<()> {
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if meta.len() < MAX_LOG_SIZE {
        return Ok(());
    }
    rotate(path)
}

/// Force-rotate the log file at `path`, regardless of its size.
///
/// The previous `<path>.1.log` becomes `<path>.2.log`; the chain proceeds
/// up to [`MAX_LOG_FILES`], and the oldest is deleted.
pub fn rotate(path: &Path) -> io::Result<()> {
    // 1) Drop the oldest file if present.
    let oldest = rotated_path(path, MAX_LOG_FILES);
    if oldest.exists() {
        std::fs::remove_file(&oldest)?;
    }
    // 2) Walk the rest of the chain forward.
    for i in (1..MAX_LOG_FILES).rev() {
        let from = rotated_path(path, i);
        let to = rotated_path(path, i + 1);
        if from.exists() {
            std::fs::rename(&from, &to)?;
        }
    }
    // 3) Move the live log to .1.
    if path.exists() {
        let first = rotated_path(path, 1);
        std::fs::rename(path, &first)?;
    }
    Ok(())
}

/// Return the path of the Nth rotated log file (N=1 → `<log>.1.log`, etc.).
pub fn rotated_path(path: &Path, index: usize) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    // Insert ".N" before the extension.
    let (base, ext) = match path.file_stem().and_then(|s| s.to_str()) {
        Some(stem) => {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("log");
            (stem.to_string(), ext.to_string())
        }
        None => ("log".to_string(), "log".to_string()),
    };
    let name = format!("{base}.{index}.{ext}");
    parent.join(name)
}

/// Delete the live log file and all of its rotated siblings.
pub fn purge(path: &Path) -> io::Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    for i in 1..=MAX_LOG_FILES {
        let p = rotated_path(path, i);
        if p.exists() {
            std::fs::remove_file(&p)?;
        }
    }
    Ok(())
}

/// Total disk usage of the live log + all rotated siblings, in bytes.
pub fn total_size(path: &Path) -> u64 {
    let live = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let rotated: u64 = (1..=MAX_LOG_FILES)
        .map(|i| {
            std::fs::metadata(rotated_path(path, i))
                .map(|m| m.len())
                .unwrap_or(0)
        })
        .sum();
    live + rotated
}

/// Append `bytes` to the log file at `path`, rotating if needed.
pub fn append_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut f = open_for_append(path)?;
    f.write_all(bytes)?;
    f.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_log() -> (tempfile::TempDir, PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("app.log");
        (d, p)
    }

    #[test]
    fn rotated_path_inserts_index() {
        let p = std::path::PathBuf::from("/var/log/portless/app.log");
        assert_eq!(
            rotated_path(&p, 1),
            std::path::PathBuf::from("/var/log/portless/app.1.log")
        );
        assert_eq!(
            rotated_path(&p, 2),
            std::path::PathBuf::from("/var/log/portless/app.2.log")
        );
    }

    #[test]
    fn no_rotation_below_threshold() {
        let (_d, p) = tmp_log();
        std::fs::write(&p, b"hello").unwrap();
        rotate_if_needed(&p).unwrap();
        assert!(p.exists());
        assert!(!rotated_path(&p, 1).exists());
    }

    #[test]
    fn rotation_moves_live_to_dot_one() {
        let (_d, p) = tmp_log();
        std::fs::write(&p, vec![b'x'; (MAX_LOG_SIZE + 1) as usize]).unwrap();
        rotate_if_needed(&p).unwrap();
        assert!(!p.exists(), "live log should be gone after rotation");
        assert!(rotated_path(&p, 1).exists());
    }

    #[test]
    fn rotation_chains_through_multiple_files() {
        let (_d, p) = tmp_log();
        // Pre-create rotated files.
        std::fs::write(&p, vec![b'x'; (MAX_LOG_SIZE + 1) as usize]).unwrap();
        std::fs::write(rotated_path(&p, 1), vec![b'1'; 100]).unwrap();
        std::fs::write(rotated_path(&p, 2), vec![b'2'; 100]).unwrap();
        std::fs::write(rotated_path(&p, 3), vec![b'3'; 100]).unwrap();
        rotate_if_needed(&p).unwrap();
        // The oldest (.3) should be gone; .2 should have moved to .3.
        assert!(!rotated_path(&p, 4).exists());
        assert!(rotated_path(&p, 3).exists());
        // The content of the previous .2 is now in .3.
        let prev = std::fs::read(rotated_path(&p, 3)).unwrap();
        assert!(prev.iter().all(|b| *b == b'2'));
    }

    #[test]
    fn purge_removes_live_and_rotated() {
        let (_d, p) = tmp_log();
        std::fs::write(&p, b"x").unwrap();
        std::fs::write(rotated_path(&p, 1), b"1").unwrap();
        std::fs::write(rotated_path(&p, 2), b"2").unwrap();
        purge(&p).unwrap();
        assert!(!p.exists());
        for i in 1..=MAX_LOG_FILES {
            assert!(!rotated_path(&p, i).exists(), "rotated {i} should be gone");
        }
    }

    #[test]
    fn total_size_sums_live_and_rotated() {
        let (_d, p) = tmp_log();
        std::fs::write(&p, vec![b'x'; 100]).unwrap();
        std::fs::write(rotated_path(&p, 1), vec![b'1'; 200]).unwrap();
        std::fs::write(rotated_path(&p, 2), vec![b'2'; 300]).unwrap();
        assert_eq!(total_size(&p), 600);
    }

    #[test]
    fn append_bytes_writes_and_creates() {
        let (_d, p) = tmp_log();
        append_bytes(&p, b"first").unwrap();
        append_bytes(&p, b"second").unwrap();
        let s = std::fs::read_to_string(&p).unwrap();
        assert_eq!(s, "firstsecond");
    }

    #[test]
    fn open_for_append_creates_parent() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("sub/dir/app.log");
        let _f = open_for_append(&p).unwrap();
        assert!(p.exists());
    }
}

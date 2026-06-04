//! `/etc/hosts` synchronization.

use crate::common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The marker that surrounds the Portless-managed block.
pub const BLOCK_BEGIN: &str = "# >>> portless >>> begin";
/// The marker that ends the Portless-managed block.
pub const BLOCK_END: &str = "# <<< portless <<< end";

/// The location of the hosts file for the current platform.
pub fn hosts_path() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from("/etc/hosts")
    }
    #[cfg(windows)]
    {
        // %WINDIR%\System32\drivers\etc\hosts
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        PathBuf::from(windir)
            .join("System32")
            .join("drivers")
            .join("etc")
            .join("hosts")
    }
}

/// A single line in the hosts file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostsLine {
    /// IP address.
    pub ip: String,
    /// Hostnames.
    pub hostnames: Vec<String>,
}

impl HostsLine {
    /// Parse a single hosts line.
    pub fn parse(line: &str) -> Option<Self> {
        let line = line.split('#').next()?.trim();
        if line.is_empty() {
            return None;
        }
        let mut parts = line.split_whitespace();
        let ip = parts.next()?.to_string();
        let hostnames: Vec<String> = parts.map(String::from).collect();
        if hostnames.is_empty() {
            return None;
        }
        Some(Self { ip, hostnames })
    }

    /// Format as a hosts line.
    pub fn format(&self) -> String {
        format!("{} {}", self.ip, self.hostnames.join(" "))
    }
}

/// A managed block of entries (always writes a single block).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HostsBlock {
    /// Lines.
    pub lines: Vec<HostsLine>,
}

impl HostsBlock {
    /// Render as a string block.
    pub fn render(&self) -> String {
        let mut s = String::new();
        s.push_str(BLOCK_BEGIN);
        s.push('\n');
        s.push_str("# DO NOT EDIT -- managed by `portless`. See `portless hosts`.\n");
        for line in &self.lines {
            s.push_str(&line.format());
            s.push('\n');
        }
        s.push_str(BLOCK_END);
        s.push('\n');
        s
    }

    /// Parse a block out of an existing hosts file content.
    pub fn extract(content: &str) -> Option<(usize, usize, Self)> {
        let start = content.find(BLOCK_BEGIN)?;
        let end_marker_start = content.find(BLOCK_END)?;
        if end_marker_start < start {
            return None;
        }
        let body_start = start + BLOCK_BEGIN.len();
        let body = &content[body_start..end_marker_start];
        let mut lines = vec![];
        for ln in body.lines() {
            if let Some(parsed) = HostsLine::parse(ln) {
                lines.push(parsed);
            }
        }
        Some((start, end_marker_start + BLOCK_END.len(), Self { lines }))
    }
}

/// Sync the given entries into the hosts file. Existing Portless entries are
/// replaced.
pub async fn sync(entries: Vec<HostsLine>) -> Result<()> {
    let path = hosts_path();
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| Error::Hosts(format!("read {}: {e}", path.display())))?;
    let new_content = replace_block(&content, &entries);
    if new_content == content {
        return Ok(());
    }
    tokio::fs::write(&path, new_content)
        .await
        .map_err(|e| Error::Hosts(format!("write {}: {e}", path.display())))
}

/// Remove the Portless-managed block from the hosts file.
pub async fn clean() -> Result<()> {
    let path = hosts_path();
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(Error::Hosts(format!("read {}: {e}", path.display()))),
    };
    let new_content = replace_block(&content, &[]);
    tokio::fs::write(&path, new_content)
        .await
        .map_err(|e| Error::Hosts(format!("write {}: {e}", path.display())))
}

fn replace_block(content: &str, entries: &[HostsLine]) -> String {
    let (start, end, _) = match HostsBlock::extract(content) {
        Some(v) => v,
        None => {
            // No existing block; append one (with a blank line separator).
            if entries.is_empty() {
                return content.to_string();
            }
            let mut s = content.trim_end().to_string();
            s.push('\n');
            s.push('\n');
            let block = HostsBlock {
                lines: entries.to_vec(),
            };
            s.push_str(&block.render());
            return s;
        }
    };
    if entries.is_empty() {
        let mut s = String::with_capacity(content.len());
        s.push_str(&content[..start]);
        // Trim any trailing whitespace before the block.
        while s.ends_with(' ') || s.ends_with('\n') || s.ends_with('\t') {
            s.pop();
        }
        s.push('\n');
        s.push_str(content[end..].trim_start_matches('\n'));
        s
    } else {
        let block = HostsBlock {
            lines: entries.to_vec(),
        };
        let mut s = String::with_capacity(content.len() + 64);
        s.push_str(&content[..start]);
        s.push_str(&block.render());
        s.push_str(&content[end..]);
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_line_basic() {
        let l = HostsLine::parse("127.0.0.1 myapp.localhost api.myapp.localhost").unwrap();
        assert_eq!(l.ip, "127.0.0.1");
        assert_eq!(l.hostnames, vec!["myapp.localhost", "api.myapp.localhost"]);
    }

    #[test]
    fn parse_line_ignores_comments() {
        let l = HostsLine::parse("127.0.0.1 myapp.localhost # comment").unwrap();
        assert_eq!(l.hostnames, vec!["myapp.localhost"]);
    }

    #[test]
    fn format_round_trip() {
        let l = HostsLine {
            ip: "127.0.0.1".into(),
            hostnames: vec!["a".into(), "b".into()],
        };
        let s = l.format();
        let p = HostsLine::parse(&s).unwrap();
        assert_eq!(p, l);
    }

    #[test]
    fn extract_block() {
        let content = format!(
            "127.0.0.1 localhost\n{}\n127.0.0.1 myapp # myapp\n{}\n",
            BLOCK_BEGIN, BLOCK_END
        );
        let (_s, _e, block) = HostsBlock::extract(&content).unwrap();
        assert_eq!(block.lines.len(), 1);
        assert_eq!(block.lines[0].ip, "127.0.0.1");
    }

    #[test]
    fn replace_block_empty() {
        let content = "127.0.0.1 localhost\n";
        let new = replace_block(content, &[]);
        assert_eq!(new, content);
    }

    #[test]
    fn replace_block_insert() {
        let content = "127.0.0.1 localhost\n";
        let entries = vec![HostsLine {
            ip: "127.0.0.1".into(),
            hostnames: vec!["myapp.localhost".into()],
        }];
        let new = replace_block(content, &entries);
        assert!(new.contains(BLOCK_BEGIN));
        assert!(new.contains("myapp.localhost"));
    }

    #[test]
    fn replace_block_remove() {
        let content = format!(
            "127.0.0.1 localhost\n{}\n127.0.0.1 myapp\n{}\n",
            BLOCK_BEGIN, BLOCK_END
        );
        let new = replace_block(&content, &[]);
        assert!(!new.contains(BLOCK_BEGIN));
        assert!(new.contains("127.0.0.1 localhost"));
    }
}

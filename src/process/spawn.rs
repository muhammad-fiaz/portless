//! Spawning user commands with proxy-aware environment variables.
//!
//! ## Platform Notes
//!
//! ### Windows
//! On Windows, many common tools (`npm`, `pnpm`, `yarn`, `bun`, `deno`, etc.)
//! are installed as `.cmd` or `.bat` wrapper scripts, not native executables.
//! Spawning them directly via `tokio::process::Command` fails with
//! `program not found` because Windows cannot locate `.cmd` files unless the
//! command is run through `cmd.exe`. This module automatically wraps every
//! command in `cmd.exe /d /s /c "..."` on Windows, matching the behaviour of
//! Node.js `child_process.spawn({ shell: true })`.
//!
//! ### Unix
//! On Unix, shell scripts and version-manager shims (nvm, fnm, mise, asdf)
//! require `/bin/sh -c "..."` so that PATH and shell functions are resolved
//! correctly.

use crate::common::{Error, Result};
use crate::process::child::Child;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

/// Builder for spawning a user command.
#[derive(Debug, Clone)]
pub struct Spawner {
    program: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    env: Vec<(String, String)>,
    remove_env: Vec<String>,
    log: Option<PathBuf>,
    force_color: bool,
    /// Whether to inherit stdio from the parent (terminal mode).
    inherit_stdio: bool,
}

impl Spawner {
    /// Construct a new spawner for the given program.
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: vec![],
            cwd: None,
            env: vec![],
            remove_env: vec![],
            log: None,
            force_color: false,
            inherit_stdio: true,
        }
    }

    /// Add a positional argument.
    pub fn arg(mut self, a: impl Into<String>) -> Self {
        self.args.push(a.into());
        self
    }

    /// Add multiple positional arguments.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for a in args {
            self.args.push(a.into());
        }
        self
    }

    /// Set the working directory.
    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Add or overwrite an environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let key = key.into();
        self.remove_env.retain(|k| k != &key);
        self.env.push((key, value.into()));
        self
    }

    /// Add many environment variables.
    pub fn envs<I, K, V>(mut self, iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in iter {
            self = self.env(k, v);
        }
        self
    }

    /// Remove an environment variable from the child.
    pub fn env_remove(mut self, key: impl Into<String>) -> Self {
        let key = key.into();
        self.env.retain(|(k, _)| k != &key);
        self.remove_env.push(key);
        self
    }

    /// Capture child output to a log file instead of inheriting stdio.
    pub fn log_to(mut self, path: impl Into<PathBuf>) -> Self {
        self.log = Some(path.into());
        self.inherit_stdio = false;
        self
    }

    /// Force ANSI color in the child even when no TTY is attached.
    pub fn force_color(mut self, force: bool) -> Self {
        self.force_color = force;
        self
    }

    /// Whether to inherit stdio from the parent process (default: true).
    ///
    /// When `true` the child shares the parent's terminal (stdin/stdout/stderr).
    /// When `false` stdio is piped (use together with [`Self::log_to`]).
    pub fn inherit_stdio(mut self, inherit: bool) -> Self {
        self.inherit_stdio = inherit;
        self
    }

    /// Build a platform-appropriate `tokio::process::Command`.
    ///
    /// On Windows every command is wrapped in `cmd.exe /d /s /c` so that
    /// `.cmd`/`.bat` wrappers (npm, pnpm, yarn, bun, …) are found on PATH.
    /// On Unix the command is wrapped in `/bin/sh -c` for compatibility with
    /// shell scripts and version-manager shims.
    pub fn build_command(&self) -> Result<Command> {
        // Deduplicate PATH entries to avoid overly long environment variables.
        let deduped_path = dedup_path();

        #[cfg(windows)]
        let mut cmd = build_windows_cmd(&self.program, &self.args);
        #[cfg(not(windows))]
        let mut cmd = build_unix_cmd(&self.program, &self.args);

        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }

        // Inherit the parent environment, apply PATH deduplication, then allow
        // explicit overrides.
        cmd.env_clear()
            .envs(std::env::vars().filter(|(k, _)| !self.remove_env.contains(k)))
            .envs(self.env.iter().map(|(k, v)| (k.as_str(), v.as_str())));

        if let Some(p) = deduped_path {
            cmd.env("PATH", p);
        }

        if self.force_color {
            cmd.env("FORCE_COLOR", "1");
            cmd.env("CLICOLOR_FORCE", "1");
        }

        if self.inherit_stdio {
            cmd.stdin(Stdio::inherit());
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::inherit());
        } else {
            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
        }

        Ok(cmd)
    }

    /// Spawn the child. Returns a [`Child`] handle.
    pub async fn spawn(self) -> Result<Child> {
        if self.program.is_empty() {
            return Err(Error::ProcessNotFound("empty program name".into()));
        }
        let mut cmd = self.build_command()?;
        if let Some(log) = &self.log {
            // Open the log file in append mode (Stdio::from needs a
            // std::fs::File, not a tokio::fs::File). Rotate first if the
            // existing log has grown past `MAX_LOG_SIZE`.
            let f = crate::process::logs::open_for_append(log)?;
            cmd.stdout(std::process::Stdio::from(f.try_clone()?));
            cmd.stderr(std::process::Stdio::from(f));
        }
        let child = cmd.spawn().map_err(|e| {
            // Provide an actionable error message.
            let prog = &self.program;
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::ProcessNotFound(format!(
                    "'{prog}' not found on PATH. \
                     Make sure it is installed and available in your shell."
                ))
            } else {
                Error::Io(e)
            }
        })?;
        Ok(Child::new(child))
    }
}

/// On Windows, wrap the command in `cmd.exe /d /s /c "..."`.
///
/// This is required so that `.cmd` and `.bat` wrappers (npm, pnpm, yarn, bun,
/// deno, etc.) are found by the OS. Without this they fail with ENOENT.
#[cfg(windows)]
fn build_windows_cmd(program: &str, args: &[String]) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.arg("/d");
    cmd.arg("/s");
    cmd.arg("/c");
    // Build the full command line as a single string, quoting each token.
    let mut full = quote_cmd_arg(program);
    for a in args {
        full.push(' ');
        full.push_str(&quote_cmd_arg(a));
    }
    cmd.arg(full);
    cmd
}

/// Quote a single token for use in a `cmd.exe /c` command line.
///
/// This is a minimal quoting strategy: wrap in double-quotes and escape
/// embedded double-quotes. For most tool invocations this is sufficient.
#[cfg(windows)]
fn quote_cmd_arg(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".to_string();
    }
    // If the token has no special characters we can use it verbatim.
    let needs_quote = s
        .chars()
        .any(|c| matches!(c, ' ' | '\t' | '"' | '&' | '|' | '<' | '>' | '^'));
    if !needs_quote {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        if ch == '"' {
            out.push('\\');
        }
        out.push(ch);
    }
    out.push('"');
    out
}

/// On Unix, wrap the command in `/bin/sh -c "..."`.
///
/// This resolves shell scripts and version-manager shims (nvm, fnm, mise,
/// asdf) and respects `node_modules/.bin` entries that are symlinks to shell
/// scripts.
#[cfg(not(windows))]
fn build_unix_cmd(program: &str, args: &[String]) -> Command {
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c");
    // Prefix node_modules/.bin to PATH so local project binaries (next, vite,
    // etc.) are found without a global install.
    let local_bin = "./node_modules/.bin";
    let path_prefix = format!("{local_bin}:");
    let mut full = format!("PATH=\"{path_prefix}$PATH\" ");
    full.push_str(&shell_escape(program));
    for a in args {
        full.push(' ');
        full.push_str(&shell_escape(a));
    }
    cmd.arg(full);
    cmd
}

/// Minimal POSIX shell word quoting.
#[cfg(not(windows))]
fn shell_escape(s: &str) -> String {
    // Single-quote the string and escape any embedded single-quotes by
    // ending the single-quoted section, inserting a literal `'`, and
    // reopening.
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Deduplicate the PATH environment variable (Windows accumulates duplicates
/// when version managers mutate it across nested processes).
fn dedup_path() -> Option<String> {
    let path = std::env::var("PATH").ok()?;
    let sep = if cfg!(windows) { ';' } else { ':' };
    let mut seen = std::collections::LinkedList::new();
    let mut set = std::collections::HashSet::new();
    for entry in path.split(sep) {
        let e = entry.trim();
        if !e.is_empty() && set.insert(e.to_ascii_lowercase()) {
            seen.push_back(e.to_string());
        }
    }
    let deduped: Vec<String> = seen.into_iter().collect();
    Some(deduped.join(&sep.to_string()))
}

/// Compatibility: when the user sets `PORTLESS=0`, the command runs
/// unmodified (via shell) through `tokio::process::Command::status()`.
pub async fn run_direct(
    program: &str,
    args: &[String],
    cwd: Option<&std::path::Path>,
    env: &[(String, String)],
) -> Result<std::process::ExitStatus> {
    #[cfg(windows)]
    let mut cmd = build_windows_cmd(program, args);
    #[cfg(not(windows))]
    let mut cmd = build_unix_cmd(program, args);

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    cmd.env_clear()
        .envs(std::env::vars())
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    let status = cmd.status().await?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn spawn_existing_program() {
        // Use the shell directly to run a trivial command on both platforms.
        // On Windows we run `cmd /c exit 0`; on Unix we run `/bin/sh -c true`.
        #[cfg(unix)]
        let status = run_direct("/bin/sh", &["-c".to_string(), "true".to_string()], None, &[])
            .await
            .unwrap();
        #[cfg(windows)]
        let status = {
            // On Windows run_direct goes through cmd.exe, so just pick any
            // program that reliably exists: cmd itself, with /c exit 0.
            let mut cmd = Command::new("cmd");
            cmd.arg("/d").arg("/s").arg("/c").arg("exit 0");
            cmd.status().await.unwrap()
        };
        assert!(status.success());
    }

    #[cfg(windows)]
    #[test]
    fn windows_quote_no_spaces() {
        assert_eq!(quote_cmd_arg("hello"), "hello");
    }

    #[cfg(windows)]
    #[test]
    fn windows_quote_with_spaces() {
        assert_eq!(quote_cmd_arg("hello world"), "\"hello world\"");
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_shell_escape_basic() {
        assert_eq!(shell_escape("hello"), "'hello'");
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_shell_escape_with_quote() {
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }
}


//! GitHub issue reporting: panic hook + structured error reporter.
//!
//! Whenever `portless` hits a condition it cannot recover from, this module
//! prints a short, copy-pasteable message that includes a pre-filled
//! `https://github.com/muhammad-fiaz/portless/issues/new?...` URL so the
//! user can file a report in one click.
//!
//! Three entry points are exposed:
//!
//! - [`install_panic_hook`] – registers a panic hook that calls
//!   [`print_panic_message`] on any uncaught panic. Idempotent.
//! - [`print_panic_message`] – prints a panic report to stderr.
//! - [`Error::report`] – pretty-prints a recoverable error and an issue URL.

use crate::common::ISSUES_URL;
use crate::common::error::Error;
#[cfg(test)]
use crate::common::{REPO_NAME, REPO_OWNER};
use std::sync::Once;

/// Install a global panic hook that prints a friendly bug-report message.
///
/// This is safe to call multiple times; only the first call has any effect.
pub fn install_panic_hook() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // Preserve the default Rust panic message (location + backtrace).
            previous(info);
            print_panic_message(info);
        }));
    });
}

/// Print a panic report to stderr and a pre-filled issue URL.
pub fn print_panic_message(info: &std::panic::PanicHookInfo<'_>) {
    let payload = if let Some(s) = info.payload().downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "non-string panic payload".to_string()
    };
    let location = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "unknown location".to_string());

    let version = env!("CARGO_PKG_VERSION");
    let triple = host_triple();
    let title = format!("panic: {first_line}", first_line = first_line(&payload));
    let body = build_issue_body(
        "panic",
        &payload,
        Some(&location),
        Some(version),
        triple.as_deref(),
    );
    let url = build_issue_url(&title, &body, &["bug", "panic"]);

    eprintln!();
    eprintln!("============================================================");
    eprintln!("portless hit an unexpected internal error (a panic).");
    eprintln!("This is a bug. Please file a report:");
    eprintln!("  {url}");
    eprintln!();
    eprintln!("  payload:  {payload}");
    eprintln!("  location: {location}");
    eprintln!("  version:  {version}");
    eprintln!("  host:     {}", triple.as_deref().unwrap_or("unknown"));
    eprintln!("============================================================");
}

impl Error {
    /// Pretty-print this error to stderr with an actionable hint and a
    /// pre-filled GitHub issue URL.
    ///
    /// Used at the top-level error boundary in `main` to give the user
    /// one-click access to a bug report without dumping a stack trace.
    pub fn report(&self) {
        let hint = self.hint();
        eprintln!();
        eprintln!("portless: {self}");
        if let Some(h) = hint {
            eprintln!("hint:     {h}");
        }
        if self.is_reportable() {
            let title = format!("error: {self}");
            let body = build_issue_body(
                "error",
                &self.to_string(),
                None,
                Some(env!("CARGO_PKG_VERSION")),
                host_triple().as_deref(),
            );
            let url = build_issue_url(&title, &body, &["bug"]);
            eprintln!();
            eprintln!("If this looks like a bug, please file a report:");
            eprintln!("  {url}");
        }
    }

    /// Process exit code associated with this error.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Config(_)
            | Self::InvalidHostname(_)
            | Self::InvalidTld(_)
            | Self::RouteNotFound(_)
            | Self::RouteExists(_)
            | Self::PortInUse(_)
            | Self::PortNotAssignable(_, _)
            | Self::NoFreePort(_, _)
            | Self::Permission(_)
            | Self::ProcessNotFound(_)
            | Self::UnsupportedPlatform(_)
            | Self::AddrParse(_)
            | Self::ReservedTld(_, _) => 2,
            _ => 1,
        }
    }

    /// Whether a one-line `hint` is available for this error.
    pub fn hint(&self) -> Option<String> {
        Some(match self {
            Self::RouteExists(_) => {
                "run `portless run --force <cmd>` to take over, or use a different name".to_string()
            }
            Self::PortInUse(_) => {
                "portless picks ports in 4000-4999; set PORTLESS_APP_PORT to pin a different port".into()
            }
            Self::Permission(_) => {
                "port 443 / 80 require elevated privileges; use `--port 1355` for a non-privileged port".into()
            }
            Self::Tls(_) => "run `portless trust` to install the local CA, or use `--no-tls`".into(),
            Self::Proxy(_) => "is the proxy running? try `portless proxy start`".into(),
            Self::RouteNotFound(_) => "is the app running? check `portless list`".into(),
            Self::LoopDetected(_) => "set `changeOrigin: true` in your framework's dev proxy config".into(),
            Self::Cancelled => "operation was cancelled (Ctrl+C)".into(),
            Self::Timeout(_) => "operation took too long; check your network and try again".into(),
            _ => return None,
        })
    }

    /// True for errors that look like bugs and should be reported.
    pub fn is_reportable(&self) -> bool {
        matches!(
            self,
            Self::StateCorruption(_)
                | Self::Lock(_)
                | Self::LoopDetected(_)
                | Self::NotImplemented(_)
                | Self::Bind { .. }
                | Self::Other(_)
        )
    }
}

fn build_issue_body(
    kind: &str,
    message: &str,
    location: Option<&str>,
    version: Option<&str>,
    host_triple: Option<&str>,
) -> String {
    let mut body = String::new();
    body.push_str(&format!("### What happened\n\n{kind}: `{message}`\n\n"));
    if let Some(loc) = location {
        body.push_str(&format!("**location:** `{loc}`\n"));
    }
    if let Some(v) = version {
        body.push_str(&format!("**portless version:** `{v}`\n"));
    }
    if let Some(t) = host_triple {
        body.push_str(&format!("**host:** `{t}`\n"));
    }
    body.push_str("\n### Steps to reproduce\n\n1. \n2. \n3. \n\n");
    body.push_str("### Expected behaviour\n\n\n");
    body.push_str("### Actual behaviour\n\n\n");
    body.push_str("### Logs / extra context\n\n```\n\n```\n");
    body
}

fn build_issue_url(title: &str, body: &str, labels: &[&str]) -> String {
    // The `new` endpoint accepts query parameters to pre-fill the form.
    // GitHub truncates the URL to ~8KB so we cap the body length.
    let title = truncate(title, 200);
    let body = truncate(body, 6000);
    let labels = labels.join(",");
    let params = [
        ("title", title.as_str()),
        ("body", body.as_str()),
        ("labels", labels.as_str()),
    ];
    let mut url = format!("{ISSUES_URL}/new?");
    for (i, (k, v)) in params.iter().enumerate() {
        if i > 0 {
            url.push('&');
        }
        url.push_str(k);
        url.push('=');
        url.push_str(&percent_encode(v));
    }
    url
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        // Truncate at a char boundary.
        let mut end = max;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

/// Minimal RFC 3986 percent-encoder for the issue-URL parameters.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        let c = *b;
        let safe = c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.' | b'_' | b'~');
        if safe {
            out.push(c as char);
        } else {
            out.push_str(&format!("%{c:02X}"));
        }
    }
    out
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s)
}

/// Best-effort host triple. Cached for the lifetime of the process.
fn host_triple() -> Option<String> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        Some("linux/x86_64".to_string())
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        Some("linux/aarch64".to_string())
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        Some("macos/x86_64".to_string())
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        Some("macos/aarch64".to_string())
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        Some("windows/x86_64".to_string())
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        Some("windows/aarch64".to_string())
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
    )))]
    {
        None
    }
}

/// Sanity-check the URL builder by hand-decoding the `body` query parameter.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_url_contains_required_params() {
        let url = build_issue_url("test title", "test body", &["bug", "panic"]);
        assert!(url.starts_with(&format!("{}/new?", ISSUES_URL)));
        assert!(url.contains("title=test%20title"));
        assert!(url.contains("body=test%20body"));
        assert!(url.contains("labels=bug%2Cpanic"));
    }

    #[test]
    fn percent_encode_handles_spaces_and_newlines() {
        assert_eq!(percent_encode("a b"), "a%20b");
        assert_eq!(percent_encode("a\nb"), "a%0Ab");
        assert_eq!(percent_encode("a/b"), "a%2Fb");
        assert_eq!(percent_encode("a-b_~."), "a-b_~.");
    }

    #[test]
    fn truncate_caps_long_strings() {
        let s = "a".repeat(10_000);
        // `truncate(_, 100)` returns 100 input bytes + the 3-byte "…" suffix.
        let out = truncate(&s, 100);
        assert!(out.ends_with('…'));
        assert!(out.len() <= 103);
    }

    #[test]
    fn first_line_picks_first_segment() {
        assert_eq!(first_line("hello\nworld"), "hello");
        assert_eq!(first_line("only one"), "only one");
    }

    #[test]
    fn error_report_does_not_panic() {
        // Just exercise the format path; output goes to stderr.
        let e = Error::Config("test".into());
        e.report();
    }

    #[test]
    fn hint_surfaces_for_known_errors() {
        assert!(Error::RouteExists("h".into()).hint().is_some());
        assert!(Error::Permission("p".into()).hint().is_some());
        assert!(Error::Cancelled.hint().is_some());
        assert!(Error::Other(anyhow::anyhow!("x")).hint().is_none());
    }

    #[test]
    fn reportable_matches_documented_set() {
        assert!(Error::LoopDetected(3).is_reportable());
        assert!(Error::StateCorruption("x".into()).is_reportable());
        assert!(!Error::Cancelled.is_reportable());
        assert!(!Error::Config("x".into()).is_reportable());
    }

    #[test]
    fn exit_codes_are_nonzero_for_failures() {
        assert_ne!(Error::Cancelled.exit_code(), 0);
        assert_ne!(Error::RouteExists("h".into()).exit_code(), 0);
    }

    #[test]
    fn repo_constants_are_correct() {
        assert_eq!(REPO_OWNER, "muhammad-fiaz");
        assert_eq!(REPO_NAME, "portless");
        assert!(ISSUES_URL.starts_with("https://github.com/"));
    }
}

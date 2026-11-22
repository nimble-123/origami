//! Output: diagnostics in every format go to stderr; stdout carries only
//! formatted text (stdin mode) or the diff (`--diff`).

pub mod agent;
pub mod diff;
pub mod human;
pub mod json;
pub mod sarif;

use origami_core::{Language, Reported, Status};

/// The outcome for one file (or `<stdin>`, or a config file).
#[derive(Debug, Clone)]
pub struct FileReport {
    pub path: String,
    pub language: Option<Language>,
    pub changed: bool,
    pub status: Status,
    pub diagnostics: Vec<Reported>,
    /// The source text, kept for files with errors (code frames).
    pub source: Option<String>,
}

pub struct Run<'a> {
    pub files: &'a [FileReport],
    pub max_diagnostics: usize,
    pub check: bool,
}

impl Run<'_> {
    /// Diagnostics per file after the per-run cap, and whether it cut any.
    pub fn capped(&self) -> (Vec<(&FileReport, &[Reported])>, bool) {
        let mut left = self.max_diagnostics;
        let mut truncated = false;
        let out = self
            .files
            .iter()
            .map(|f| {
                let n = f.diagnostics.len().min(left);
                left -= n;
                truncated |= n < f.diagnostics.len();
                (f, &f.diagnostics[..n])
            })
            .collect();
        (out, truncated)
    }
}

/// The stream a report is written to (for color detection).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stream {
    /// Used by the review view (`--review`).
    #[allow(dead_code)]
    Stdout,
    Stderr,
}

/// Colors when the stream is a terminal; `NO_COLOR` turns them off,
/// `CLICOLOR_FORCE=1` turns them on.
pub(crate) fn color_enabled(stream: Stream) -> bool {
    use std::io::IsTerminal;
    if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        return false;
    }
    if std::env::var_os("CLICOLOR_FORCE").is_some_and(|v| v != "0") {
        return true;
    }
    match stream {
        Stream::Stdout => std::io::stdout().is_terminal(),
        Stream::Stderr => std::io::stderr().is_terminal(),
    }
}

fn needs_escape(c: char) -> bool {
    let u = c as u32;
    (u < 0x20 && c != '\t')
        || u == 0x7f
        || (0x80..=0x9f).contains(&u)
        || (0x202a..=0x202e).contains(&u)
        || (0x2066..=0x2069).contains(&u)
}

/// Source text for a terminal: control characters (C0 except tab, DEL, C1)
/// and bidi controls are shown escaped, so a file cannot send terminal escape
/// sequences or reorder the displayed text.
pub(crate) fn escape_controls(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.chars().any(needs_escape) {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if needs_escape(c) {
            let u = c as u32;
            if u < 0x100 {
                out.push_str(&format!("\\x{u:02x}"));
            } else {
                out.push_str(&format!("\\u{{{u:x}}}"));
            }
        } else {
            out.push(c);
        }
    }
    std::borrow::Cow::Owned(out)
}

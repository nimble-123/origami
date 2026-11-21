//! Output: diagnostics in every format go to stderr; stdout carries only
//! formatted text (stdin mode) or the diff (`--diff`).

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

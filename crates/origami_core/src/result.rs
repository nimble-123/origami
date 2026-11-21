//! Result types.

use origami_syntax::{Code, Diagnostic, Language, LineIndex, Position, Severity, Stats};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Formatted,
    Unchanged,
    Error,
    Skipped,
}

/// A diagnostic with line/column positions, as reported to users.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reported {
    pub code: Code,
    pub severity: Severity,
    pub message: String,
    pub range: ReportedRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportedRange {
    pub start: Position,
    pub end: Position,
}

impl Reported {
    /// A diagnostic that is not tied to a position in a source (I/O, config).
    pub fn at_start(code: Code, message: impl Into<String>) -> Reported {
        let p = Position {
            line: 1,
            column: 1,
            offset: 0,
        };
        Reported {
            code,
            severity: code.severity(),
            message: message.into(),
            range: ReportedRange { start: p, end: p },
        }
    }

    pub fn from(d: &Diagnostic, index: &LineIndex) -> Reported {
        Reported {
            code: d.code,
            severity: d.severity(),
            message: d.message.clone(),
            range: ReportedRange {
                start: index.position(d.start),
                end: index.position(d.end),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormatResult {
    /// Encoded output bytes. `None` on any error.
    pub output: Option<Vec<u8>>,
    pub changed: bool,
    pub language: Option<Language>,
    pub status: Status,
    pub diagnostics: Vec<Reported>,
    pub stats: Stats,
}

impl FormatResult {
    pub fn has(&self, code: Code) -> bool {
        self.diagnostics.iter().any(|d| d.code == code)
    }
}

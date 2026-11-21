//! Diagnostics and the complete v1 code table (see `docs/CLI.md`).

use serde::Serialize;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Internal,
}

/// Every diagnostic code of v1. Codes are stable once released.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Code {
    E001,
    E002,
    E003,
    E004,
    E005,
    E010,
    E011,
    E020,
    E021,
    E030,
    E031,
    W001,
    W002,
    W003,
    W004,
    W101,
    X001,
    X002,
}

impl Code {
    pub fn as_str(self) -> &'static str {
        match self {
            Code::E001 => "E001",
            Code::E002 => "E002",
            Code::E003 => "E003",
            Code::E004 => "E004",
            Code::E005 => "E005",
            Code::E010 => "E010",
            Code::E011 => "E011",
            Code::E020 => "E020",
            Code::E021 => "E021",
            Code::E030 => "E030",
            Code::E031 => "E031",
            Code::W001 => "W001",
            Code::W002 => "W002",
            Code::W003 => "W003",
            Code::W004 => "W004",
            Code::W101 => "W101",
            Code::X001 => "X001",
            Code::X002 => "X002",
        }
    }

    pub fn severity(self) -> Severity {
        match self.as_str().as_bytes()[0] {
            b'E' => Severity::Error,
            b'W' => Severity::Warning,
            b'X' => Severity::Internal,
            _ => Severity::Info,
        }
    }

    /// Exit code this diagnostic contributes. Warnings contribute 0, or 5 with
    /// `--strict`; the CLI applies that rule.
    pub fn exit_code(self) -> i32 {
        match self {
            Code::E001 | Code::E002 | Code::E003 | Code::E004 | Code::E005 | Code::E010 => 2,
            Code::E011 | Code::E020 | Code::E021 => 3,
            Code::E030 | Code::E031 => 4,
            Code::X001 | Code::X002 => 70,
            Code::W001 | Code::W002 | Code::W003 | Code::W004 | Code::W101 => 0,
        }
    }

    pub fn all() -> &'static [Code] {
        &[
            Code::E001,
            Code::E002,
            Code::E003,
            Code::E004,
            Code::E005,
            Code::E010,
            Code::E011,
            Code::E020,
            Code::E021,
            Code::E030,
            Code::E031,
            Code::W001,
            Code::W002,
            Code::W003,
            Code::W004,
            Code::W101,
            Code::X001,
            Code::X002,
        ]
    }
}

impl Serialize for Code {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

/// A diagnostic with a byte range in the normalized source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: Code,
    pub message: String,
    pub start: u32,
    pub end: u32,
}

impl Diagnostic {
    pub fn new(code: Code, message: impl Into<String>, range: Range<u32>) -> Diagnostic {
        Diagnostic {
            code,
            message: message.into(),
            start: range.start,
            end: range.end,
        }
    }

    pub fn severity(&self) -> Severity {
        self.code.severity()
    }
}

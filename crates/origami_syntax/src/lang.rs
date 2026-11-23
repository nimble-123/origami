//! Types shared between `origami_core` and the language crates. They live here
//! so that the language crates do not depend on the core.

use crate::diagnostic::Diagnostic;
use crate::explain::RuleSpan;
use crate::token::{TokenIndex, TokenStream};
use serde::Serialize;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Abap,
    Ddls,
    Ddlx,
    Dcl,
    Bdef,
    Srvd,
}

impl Language {
    pub fn as_str(self) -> &'static str {
        match self {
            Language::Abap => "abap",
            Language::Ddls => "ddls",
            Language::Ddlx => "ddlx",
            Language::Dcl => "dcl",
            Language::Bdef => "bdef",
            Language::Srvd => "srvd",
        }
    }

    pub fn parse(s: &str) -> Option<Language> {
        Some(match s {
            "abap" => Language::Abap,
            "ddls" => Language::Ddls,
            "ddlx" => Language::Ddlx,
            "dcl" => Language::Dcl,
            "bdef" => Language::Bdef,
            "srvd" => Language::Srvd,
            _ => return None,
        })
    }

    pub fn is_cds_family(self) -> bool {
        self != Language::Abap
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Case {
    Upper,
    Lower,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    E2026,
}

impl Edition {
    pub fn as_str(self) -> &'static str {
        match self {
            Edition::E2026 => "2026",
        }
    }

    pub fn parse(s: &str) -> Option<Edition> {
        match s {
            "2026" => Some(Edition::E2026),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangOptions {
    pub line_width: u16,
    pub indent_width: u8,
    pub abap_keyword_case: Case,
    pub preview: bool,
}

impl Default for LangOptions {
    fn default() -> LangOptions {
        LangOptions {
            line_width: 120,
            indent_width: 2,
            abap_keyword_case: Case::Upper,
            preview: false,
        }
    }
}

pub type UnitId = u32;

/// Deepest nesting that is laid out (W004): blocks and brackets combined. A
/// unit with a token nested deeper is left as written. Real code stays far
/// below (the corpus maximum is 19); the limit bounds recursion, the size of
/// the layout and the indentation.
pub const MAX_NESTING: u32 = 100;

/// Longest output, in bytes, for an input of `len` bytes: 16 times the input
/// plus 1 MiB. Formatting real code grows it by less than 2 times; a layout
/// that would grow it beyond the limit leaves the file as written (W004).
pub fn max_output_len(len: usize) -> usize {
    len.saturating_mul(16).saturating_add(1 << 20)
}

/// The diagnostic for an output over [`max_output_len`].
pub fn output_limit_diagnostic() -> Diagnostic {
    Diagnostic::new(
        crate::Code::W004,
        "Formatting would make the file more than 16 times as large; left as written",
        0..0,
    )
}

/// The smallest piece of a source that the self-check can quarantine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub id: UnitId,
    pub tokens: Range<TokenIndex>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Stats {
    pub statements: u32,
    pub unknown: u32,
    /// Units left as written because they nest deeper than [`MAX_NESTING`].
    pub depth_limited: u32,
    pub quarantined: u32,
}

/// What a language crate returns from `format`.
#[derive(Debug, Clone)]
pub struct LangResult {
    /// Normalized output text. Meaningless when `diagnostics` holds an error.
    pub output: String,
    /// Input tokens, with keyword flags set by the parser.
    pub input_tokens: TokenStream,
    /// In source order, each id its index.
    pub units: Vec<Unit>,
    pub diagnostics: Vec<Diagnostic>,
    pub stats: Stats,
    /// Constructs and the rules that laid them out; recorded only when
    /// explanations are requested (`format_explained`).
    pub rule_spans: Vec<RuleSpan>,
}

impl LangResult {
    pub fn failed(diagnostic: Diagnostic) -> LangResult {
        LangResult {
            output: String::new(),
            input_tokens: TokenStream::default(),
            units: Vec::new(),
            diagnostics: vec![diagnostic],
            stats: Stats::default(),
            rule_spans: Vec::new(),
        }
    }

    pub fn has_error(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity() >= crate::Severity::Error)
    }

    /// The unit that contains the given input token, if any.
    pub fn unit_containing(&self, index: TokenIndex) -> Option<&Unit> {
        let k = self.units.partition_point(|u| u.tokens.end <= index);
        self.units.get(k).filter(|u| u.tokens.start <= index)
    }

    /// The unit that contains the given input token, else the last unit.
    pub fn unit_of(&self, index: TokenIndex) -> Option<UnitId> {
        self.unit_containing(index)
            .or_else(|| self.units.last())
            .map(|u| u.id)
    }
}

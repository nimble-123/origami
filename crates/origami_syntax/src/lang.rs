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

    /// The unit that contains the given input token.
    pub fn unit_of(&self, index: TokenIndex) -> Option<UnitId> {
        self.units
            .iter()
            .find(|u| u.tokens.start <= index && index < u.tokens.end)
            .map(|u| u.id)
            .or_else(|| self.units.last().map(|u| u.id))
    }
}

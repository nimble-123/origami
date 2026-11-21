//! CDS-family support for origami: DDLS, DDLX, DCL, BDEF and SRVD.
//!
//! M0 state: lexer, structural pass and units; the printer runs in identity
//! mode. Parsers arrive in M2 (DDLS, DDLX, DCL) and M3 (BDEF, SRVD). The CDS
//! printer never changes letter case.

pub mod kinds;
mod lexer;
pub mod structure;

use origami_layout::{PrintOptions, builder, print};
use origami_syntax::{LangOptions, LangResult, Language, SourceText, Stats, TokenIndex, UnitId};
use std::collections::BTreeSet;

pub use lexer::{dash_comments, lex};

pub fn format(
    src: &SourceText,
    lang: Language,
    opts: &LangOptions,
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
    identity: bool,
) -> LangResult {
    let stream = match lex(src, lang) {
        Ok(s) => s,
        Err(d) => return LangResult::failed(d),
    };
    if let Err(d) = structure::check(&stream) {
        return LangResult::failed(d);
    }
    let units = structure::units(&stream);
    let docs = units
        .iter()
        .map(|u| {
            let first = u.tokens.start;
            let last = TokenIndex(u.tokens.end.0 - 1);
            if opaque.contains(&u.id) {
                builder::opaque(first, last, true)
            } else if quarantine.contains(&u.id) {
                builder::verbatim(first, last, true)
            } else {
                builder::unknown(first, last, true)
            }
        })
        .collect();
    let print_opts = PrintOptions {
        width: opts.line_width,
        indent: opts.indent_width,
        keyword_case: None,
        identity,
    };
    let output = print(&builder::concat(docs), src, &stream, &print_opts);
    let n = units.len() as u32;
    LangResult {
        output,
        input_tokens: stream,
        units,
        diagnostics: Vec::new(),
        stats: Stats {
            statements: n,
            unknown: n,
            ..Stats::default()
        },
    }
}

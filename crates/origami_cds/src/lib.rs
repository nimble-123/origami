//! CDS-family support for origami: DDLS, DDLX, DCL, BDEF and SRVD.
//!
//! M0 state: lexer, structural pass and units; the printer runs in identity
//! mode. Parsers arrive in M2 (DDLS, DDLX, DCL) and M3 (BDEF, SRVD). The CDS
//! printer never changes letter case.

pub mod kinds;
mod lexer;
pub mod structure;

use origami_layout::{PrintOptions, builder, print};
use origami_syntax::{LangOptions, LangResult, Language, SourceText, Stats, UnitId};
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
            let range = stream.tokens[u.tokens.start.0 as usize].start
                ..stream.tokens[u.tokens.end.0 as usize - 1].end;
            if opaque.contains(&u.id) {
                builder::opaque(range)
            } else if quarantine.contains(&u.id) {
                builder::verbatim(range)
            } else {
                builder::unknown(range)
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

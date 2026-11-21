//! CDS-family support for origami: DDLS, DDLX, DCL, BDEF and SRVD.
//!
//! Layout is structural (STYLE CDS-*): one annotation or element per line,
//! indentation by `{ }` blocks, short inline blocks kept, non-key elements
//! aligned under key element names. Inside a unit, the author's line breaks
//! are kept and re-indented. The CDS printer never changes letter case.
//! Grammar-based layout of headers and expressions is future work.

pub mod kinds;
mod layout;
mod lexer;
pub mod structure;

use origami_layout::{PrintOptions, print};
use origami_syntax::{
    LangOptions, LangResult, Language, SourceText, Stats, TokenIndex, Unit, UnitId,
};
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
    let cds_units = layout::units(src.text(), &stream, lang);
    let units: Vec<Unit> = cds_units
        .iter()
        .enumerate()
        .map(|(id, u)| Unit {
            id: id as UnitId,
            tokens: TokenIndex(u.first as u32)..TokenIndex(u.last as u32 + 1),
        })
        .collect();
    let doc = layout::layout(src.text(), &stream, &cds_units, quarantine, opaque);
    let print_opts = PrintOptions {
        width: opts.line_width,
        indent: opts.indent_width,
        keyword_case: None,
        identity,
    };
    let output = print(&doc, src, &stream, &print_opts);
    let n = units.len() as u32;
    LangResult {
        output,
        input_tokens: stream,
        units,
        diagnostics: Vec::new(),
        stats: Stats {
            statements: n,
            ..Stats::default()
        },
    }
}

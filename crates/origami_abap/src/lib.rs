//! ABAP support for origami: lexer, structural pass and layout.
//!
//! M0 state: every statement is treated as unknown and the printer runs in
//! identity mode. Statement parsers arrive in M1.

pub mod block_pairs;
pub mod kinds;
mod lexer;
pub mod structure;
pub mod verbatim;

use origami_layout::{PrintOptions, builder, print};
use origami_syntax::{
    LangOptions, LangResult, SourceText, Stats, TokenIndex, TokenStream, Unit, UnitId,
};
use std::collections::BTreeSet;

pub use lexer::lex;

/// Formats ABAP source. Units in `quarantine` are printed as written
/// (re-indented only); units in `opaque` are printed byte for byte.
pub fn format(
    src: &SourceText,
    opts: &LangOptions,
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
    identity: bool,
) -> LangResult {
    let stream = match lex(src) {
        Ok(s) => s,
        Err(d) => return LangResult::failed(d),
    };
    let statements = match structure::check(src.text(), &stream) {
        Ok(s) => s,
        Err(d) => return LangResult::failed(d),
    };
    let units: Vec<Unit> = statements
        .iter()
        .enumerate()
        .map(|(id, s)| Unit {
            id: id as UnitId,
            tokens: TokenIndex(s.tokens.start as u32)..TokenIndex(s.tokens.end as u32),
        })
        .collect();

    // M0: no statement parsers yet, so every statement is an unknown statement.
    // W003 ("formatted generically") is not emitted in identity mode, because
    // nothing is formatted.
    let doc = layout(&stream, &units, quarantine, opaque);
    let print_opts = PrintOptions {
        width: opts.line_width,
        indent: opts.indent_width,
        keyword_case: Some(opts.abap_keyword_case),
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
            unknown: n,
            ..Stats::default()
        },
    }
}

fn layout(
    stream: &TokenStream,
    units: &[Unit],
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
) -> origami_layout::Doc {
    let docs = units
        .iter()
        .map(|u| {
            let first = &stream.tokens[u.tokens.start.0 as usize];
            let last = &stream.tokens[u.tokens.end.0 as usize - 1];
            let range = first.start..last.end;
            if opaque.contains(&u.id) {
                builder::opaque(range)
            } else if quarantine.contains(&u.id) {
                builder::verbatim(range)
            } else {
                builder::unknown(range)
            }
        })
        .collect();
    builder::concat(docs)
}

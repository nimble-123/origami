//! CDS-family support for origami: DDLS, DDLX, DCL, BDEF and SRVD.
//!
//! Layout is structural (STYLE CDS-*): one annotation or element per line,
//! indentation by `{ }` blocks, short inline blocks kept, non-key elements
//! aligned under key element names. Inside a unit, the author's line breaks
//! are kept and re-indented. The CDS printer never changes letter case.
//! Grammar-based layout of headers and expressions is future work.

mod grammar;
pub mod kinds;
mod layout;
mod lexer;
pub mod structure;

use origami_layout::{PrintOptions, print_bounded};
use origami_syntax::{
    LangOptions, LangResult, Language, SourceText, Stats, TokenIndex, Unit, UnitId, max_output_len,
    output_limit_diagnostic,
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
    format_with(src, lang, opts, quarantine, opaque, identity, false)
}

/// Like [`format`], and also records the rule spans for explanations
/// (`LangResult::rule_spans`). The output is the same.
pub fn format_explained(
    src: &SourceText,
    lang: Language,
    opts: &LangOptions,
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
    identity: bool,
) -> LangResult {
    format_with(src, lang, opts, quarantine, opaque, identity, true)
}

fn format_with(
    src: &SourceText,
    lang: Language,
    opts: &LangOptions,
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
    identity: bool,
    explain: bool,
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
    let laid = layout::layout(
        src.text(),
        &stream,
        lang,
        &cds_units,
        quarantine,
        opaque,
        explain,
    );
    let print_opts = PrintOptions {
        width: opts.line_width,
        indent: opts.indent_width,
        keyword_case: None,
        identity,
    };
    let limit = max_output_len(src.text().len());
    let (output, diagnostics) = match print_bounded(&laid.doc, src, &stream, &print_opts, limit) {
        // In identity mode nothing is formatted, so nothing is reported.
        Some(output) if identity => (output, Vec::new()),
        Some(output) => (output, laid.diagnostics),
        None => (src.text().to_string(), vec![output_limit_diagnostic()]),
    };
    let n = units.len() as u32;
    LangResult {
        output,
        input_tokens: stream,
        units,
        diagnostics,
        stats: Stats {
            statements: n,
            depth_limited: laid.depth_limited,
            ..Stats::default()
        },
        rule_spans: laid.rule_spans,
    }
}

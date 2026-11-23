//! ABAP support for origami: lexer, structural pass and layout.
//!
//! M1 state: statement-level layout (block indentation, one statement per
//! line, blank lines). Statements without a parser are printed as unknown
//! statements: re-indented, with their own line breaks, runs of spaces shrunk
//! to one, and W003. Statement parsers are added in order of corpus
//! frequency.

pub mod block_pairs;
pub mod kinds;
mod layout;
mod lexer;
mod statements;
mod stmt;
pub mod structure;
pub mod verbatim;

use origami_layout::{PrintOptions, print};
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
    format_with(src, opts, quarantine, opaque, identity, false)
}

/// Like [`format`], and also records the rule spans for explanations
/// (`LangResult::rule_spans`). The output is the same.
pub fn format_explained(
    src: &SourceText,
    opts: &LangOptions,
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
    identity: bool,
) -> LangResult {
    format_with(src, opts, quarantine, opaque, identity, true)
}

/// Operators that cannot be names, and constructor operators before their
/// type (`VALUE #(`, `CONV string(`).
fn always_keyword(src: &str, stream: &TokenStream, k: usize) -> bool {
    const OPERATORS: &[&str] = &[
        "AND", "OR", "NOT", "EQUIV", "EQ", "NE", "LT", "GT", "LE", "GE", "CO", "CN", "CA", "NA",
        "CS", "NS", "CP", "NP", "BETWEEN", "DIV", "MOD", "BIT-AND", "BIT-OR", "BIT-XOR", "BIT-NOT",
    ];
    let t = &stream.tokens;
    let w = t[k].text(src).to_ascii_uppercase();
    if OPERATORS.contains(&w.as_str()) {
        return true;
    }
    stmt::CTOR_OPS.contains(&w.as_str())
        && t.get(k + 2)
            .is_some_and(|p| p.kind == kinds::LPAREN && p.glued)
        && t.get(k + 1)
            .is_some_and(|ty| ty.kind == kinds::WORD && !ty.glued)
}

fn format_with(
    src: &SourceText,
    opts: &LangOptions,
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
    identity: bool,
    explain: bool,
) -> LangResult {
    let mut stream = match lex(src) {
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

    let laid = layout::layout(
        src.text(),
        &stream,
        &statements,
        quarantine,
        opaque,
        explain,
    );
    if !identity {
        for &k in &laid.keywords {
            stream.tokens[k].keyword = true;
        }
        // ABAP-CASE-002: the other words of a recognized statement are
        // identifiers, except words that are keywords wherever they stand
        // (a parser that missed one leaves it as written).
        for r in &laid.parsed {
            for k in r.clone() {
                let ident = stream.tokens[k].kind == kinds::WORD
                    && !stream.tokens[k].keyword
                    && !always_keyword(src.text(), &stream, k);
                stream.tokens[k].ident = ident;
            }
        }
    }
    let print_opts = PrintOptions {
        width: opts.line_width,
        indent: opts.indent_width,
        keyword_case: Some(opts.abap_keyword_case),
        identity,
    };
    let output = print(&laid.doc, src, &stream, &print_opts);
    // In identity mode nothing is formatted, so no W003 is reported.
    let diagnostics = if identity {
        Vec::new()
    } else {
        laid.diagnostics
    };
    LangResult {
        output,
        input_tokens: stream,
        units,
        diagnostics,
        stats: Stats {
            statements: statements.len() as u32,
            unknown: laid.unknown,
            ..Stats::default()
        },
        rule_spans: laid.rule_spans,
    }
}

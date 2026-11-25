//! ABAP support for origami: lexer, structural pass and layout.
//!
//! M1 state: statement-level layout (block indentation, one statement per
//! line, blank lines). Statements without a parser are printed as unknown
//! statements: re-indented, with their own line breaks, runs of spaces shrunk
//! to one, and W003. Statement parsers are added in order of corpus
//! frequency.

mod abap_doc;
pub mod block_pairs;
pub mod kinds;
mod layout;
mod lexer;
mod space;
mod statements;
mod stmt;
pub mod structure;
pub mod verbatim;

use origami_layout::{PrintOptions, print_bounded};
use origami_syntax::{
    LangOptions, LangResult, SourceText, Stats, Style, TokenIndex, TokenStream, Unit, UnitId,
    max_output_len, output_limit_diagnostic,
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

    // ABAP-SPACE-001: the layout sees the periods and commas glued.
    let glue = if identity {
        Vec::new()
    } else {
        space::candidates(src.text(), &stream, &statements)
    };
    let mut laid_stream = stream.clone();
    space::apply(&mut laid_stream, &glue);
    let laid = layout::layout(
        src.text(),
        &laid_stream,
        &statements,
        quarantine,
        opaque,
        explain,
        opts.style,
    );
    drop(laid_stream);
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
    // The printer's stream: statements printed as written keep their gaps,
    // and in unknown statements comments stay where they are.
    let mut print_stream = stream.clone();
    let printed: Vec<space::Glue> = glue
        .iter()
        .filter(|g| match laid.bodies[g.statement] {
            layout::Body::Parsed => true,
            layout::Body::Unknown => !g.moves,
            layout::Body::AsWritten => false,
        })
        .copied()
        .collect();
    space::apply(&mut print_stream, &printed);
    for g in &glue {
        stream.tokens[g.at].may_glue = true;
    }
    let comment_overrides = if identity {
        Default::default()
    } else {
        abap_doc::align(src.text(), &print_stream, &statements, &laid.bodies)
    };
    let print_opts = PrintOptions {
        width: opts.line_width,
        indent: opts.indent_width,
        keyword_case: Some(opts.abap_keyword_case),
        identity,
        cont_levels: match opts.style {
            Style::Aligned => 1,
            Style::Compact => 2,
        },
        comment_style: opts.style == Style::Aligned,
        blank_after_comments: laid.blank_after_comments.clone(),
        comment_overrides,
    };
    let limit = max_output_len(src.text().len());
    let (output, diagnostics) =
        match print_bounded(&laid.doc, src, &print_stream, &print_opts, limit) {
            // In identity mode nothing is formatted, so no W003 is reported.
            Some(output) if identity => (output, Vec::new()),
            Some(output) => (output, laid.diagnostics),
            None => (src.text().to_string(), vec![output_limit_diagnostic()]),
        };
    LangResult {
        output,
        input_tokens: stream,
        units,
        diagnostics,
        stats: Stats {
            statements: statements.len() as u32,
            unknown: laid.unknown,
            depth_limited: laid.depth_limited,
            ..Stats::default()
        },
        rule_spans: laid.rule_spans,
    }
}

//! ABAP-DOC-001: aligned `@parameter`, `@raising` and `@exception` lines in
//! the ABAP Doc in front of a statement.
//!
//! The result is a replacement text per comment; the printer uses the
//! replacements in front of one token only if padding pushes no line past the
//! width.

use crate::layout::Body;
use crate::structure::Statement;
use origami_syntax::compare::abap_doc_parts;
use origami_syntax::{CommentSyntax, TokenStream, TriviaKind, TriviaPosition};
use std::collections::BTreeMap;

/// Replacement texts by trivia index.
pub fn align(
    text: &str,
    stream: &TokenStream,
    statements: &[Statement],
    bodies: &[Body],
) -> BTreeMap<usize, String> {
    let mut first = vec![false; stream.tokens.len() + 1];
    for (st, body) in statements.iter().zip(bodies) {
        if *body != Body::AsWritten {
            first[st.tokens.start] = true;
        }
    }
    let mut out = BTreeMap::new();
    // The current run of `"!` lines: (trivia index, anchor).
    let mut run: Vec<usize> = Vec::new();
    let mut run_anchor = u32::MAX;
    for (k, t) in stream.trivia.iter().enumerate() {
        let TriviaKind::Comment { syntax, .. } = t.kind else {
            continue; // blank lines do not end a run
        };
        let doc = syntax == CommentSyntax::AbapDoc
            && t.position == TriviaPosition::Leading
            && first[t.anchor.0 as usize];
        if !doc || t.anchor.0 != run_anchor {
            align_run(text, stream, &run, &mut out);
            run.clear();
        }
        if doc {
            run.push(k);
            run_anchor = t.anchor.0;
        } else {
            run_anchor = u32::MAX;
        }
    }
    align_run(text, stream, &run, &mut out);
    out
}

/// Aligns the tag lines of one run of `"!` lines.
fn align_run(text: &str, stream: &TokenStream, run: &[usize], out: &mut BTreeMap<usize, String>) {
    let rows: Vec<(usize, &str, &str, &str)> = run
        .iter()
        .filter_map(|&k| {
            let (head, name, bar) = abap_doc_parts(stream.trivia[k].text(text))?;
            // As ABAP Cleaner: only lines with a text after `|`. A tab before
            // the tag would make the columns unpredictable.
            let has_text = bar[1..].chars().any(|c| !c.is_whitespace());
            (has_text && !head.contains('\t')).then_some((k, head, name, bar))
        })
        .collect();
    let width = |s: &str| s.chars().count();
    let name_col = rows.iter().map(|r| width(r.1)).max().unwrap_or(0) + 1;
    let bar_col = rows
        .iter()
        .map(|r| name_col + width(r.2))
        .max()
        .unwrap_or(0)
        + 1;
    for (k, head, name, bar) in rows {
        let aligned = format!(
            "{head}{}{name}{}{bar}",
            " ".repeat(name_col - width(head)),
            " ".repeat(bar_col - name_col - width(name)),
        );
        if aligned != stream.trivia[k].text(text) {
            out.insert(k, aligned);
        }
    }
}

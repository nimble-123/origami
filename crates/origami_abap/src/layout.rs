//! Statement-level layout (STYLE ABAP-STMT-001, ABAP-BLOCK-001,
//! ABAP-CLASS-001, ABAP-BLANK-001) and dispatch to statement parsers.
//!
//! Every statement starts on its own line at its block indent. Statements
//! without a parser are printed as unknown statements (W003). Quarantined
//! units are printed verbatim, opaque units byte for byte.

use crate::block_pairs::Role;
use crate::statements;
use crate::stmt::{Ctx, P};
use crate::structure::Statement;
use origami_layout::{Doc, builder as b};
use origami_syntax::{Code, Diagnostic, RuleSpan, TokenIndex, TokenStream, TriviaPosition, UnitId};
use std::collections::BTreeSet;

pub struct Laid {
    pub doc: Doc,
    pub diagnostics: Vec<Diagnostic>,
    pub unknown: u32,
    pub keywords: Vec<usize>,
    /// Rule spans (only when `explain`).
    pub rule_spans: Vec<RuleSpan>,
}

fn nest(level: u32, doc: Doc) -> Doc {
    (0..level).fold(doc, |d, _| b::indent(vec![d]))
}

fn opens(role: Option<Role>) -> bool {
    matches!(role, Some(Role::Opener(_)) | Some(Role::Intermediate))
}

fn closes(role: Option<Role>) -> bool {
    matches!(role, Some(Role::Closer) | Some(Role::Intermediate))
}

pub fn layout(
    text: &str,
    stream: &TokenStream,
    statements: &[Statement],
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
    explain: bool,
) -> Laid {
    let mut commented = vec![false; stream.tokens.len() + 1];
    for t in &stream.trivia {
        if t.is_comment() && t.position == TriviaPosition::Leading {
            commented[t.anchor.0 as usize] = true;
        }
    }
    let mut ctx = Ctx {
        text,
        stream,
        commented: &commented,
        keywords: Vec::new(),
        explain,
        rule_spans: Vec::new(),
    };
    let mut docs = Vec::with_capacity(statements.len());
    let mut diagnostics = Vec::new();
    let mut unknown = 0;
    let mut prev_role = None;
    for (id, st) in statements.iter().enumerate() {
        let id = id as UnitId;
        let first = TokenIndex(st.tokens.start as u32);
        let last = TokenIndex(st.tokens.end as u32 - 1);
        // One blank line is kept, except directly after an opener and
        // directly before a closer.
        let blank = id > 0 && !opens(prev_role) && !closes(st.role);
        // A statement glued to the previous one without being an opaque body
        // (invalid code such as `x.y` split at the period) keeps its bytes:
        // its position depends on the previous statement's layout, so any
        // re-indentation would not be stable.
        let glued_start = id > 0 && stream.tokens[first.0 as usize].glued;
        let body = if opaque.contains(&id)
            || (glued_start && stream.tokens[first.0 as usize].kind != crate::kinds::OPAQUE)
        {
            b::opaque(first, last, blank)
        } else if quarantine.contains(&id) {
            b::verbatim(first, last, blank)
        } else {
            let saved = ctx.mark();
            let parsed = {
                let mut p = P::new(&mut ctx, st.tokens.start, st.tokens.end - 1, blank);
                statements::parse(&mut p)
            };
            // Keywords inside constructor bodies (`FOR … IN`, `BASE`,
            // `MAPPING`, …), whichever layout the statement gets.
            let mark_ctor = |ctx: &mut Ctx| {
                let mut p = P::new(ctx, st.tokens.start, st.tokens.end - 1, blank);
                let end = p.end;
                p.mark_ctor_keywords(st.tokens.start, end, false);
            };
            match parsed {
                Some(d) => {
                    mark_ctor(&mut ctx);
                    d
                }
                None => {
                    ctx.reset(saved);
                    mark_ctor(&mut ctx);
                    unknown += 1;
                    let t = &stream.tokens[first.0 as usize];
                    diagnostics.push(Diagnostic::new(
                        Code::W003,
                        "Statement not recognized; formatted generically",
                        t.start..stream.tokens[last.0 as usize].end,
                    ));
                    b::unknown(first, last, blank)
                }
            }
        };
        // A statement glued to the previous one (an opaque AMDP, EXEC SQL or
        // macro body) continues without a line break.
        let glued = stream.tokens[first.0 as usize].glued;
        let start = if id == 0 || glued {
            b::empty()
        } else {
            b::hard(stream, first)
        };
        docs.push(nest(st.depth, b::concat(vec![start, body])));
        prev_role = st.role;
    }
    Laid {
        doc: b::concat(docs),
        diagnostics,
        unknown,
        keywords: ctx.keywords,
        rule_spans: ctx.rule_spans,
    }
}

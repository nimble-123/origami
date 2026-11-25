//! Statement-level layout (STYLE ABAP-STMT-001, ABAP-BLOCK-001,
//! ABAP-CLASS-001, ABAP-BLANK-001) and dispatch to statement parsers.
//!
//! Every statement starts on its own line at its block indent. Statements
//! without a parser are printed as unknown statements (W003). Quarantined
//! units are printed verbatim, opaque units byte for byte, and so are
//! statements nested deeper than `MAX_NESTING` (W004).

use crate::block_pairs::Role;
use crate::kinds::*;
use crate::statements;
use crate::stmt::{AlignKey, Ctx, P, Row, Stmt};
use crate::structure::Statement;
use origami_layout::{Doc, builder as b};
use origami_syntax::{
    Code, Diagnostic, MAX_NESTING, RuleSpan, Style, TokenIndex, TokenStream, TriviaPosition, UnitId,
};
use std::collections::BTreeSet;

pub struct Laid {
    pub doc: Doc,
    pub diagnostics: Vec<Diagnostic>,
    pub unknown: u32,
    pub keywords: Vec<usize>,
    /// Token ranges of recognized statements; their non-keyword words are
    /// identifiers.
    pub parsed: Vec<std::ops::Range<usize>>,
    /// Rule spans (only when `explain`).
    pub rule_spans: Vec<RuleSpan>,
    /// Statements left as written because they nest too deep (W004).
    pub depth_limited: u32,
    /// How each statement is printed.
    pub bodies: Vec<Body>,
}

/// How a statement is printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Body {
    /// Laid out by its parser.
    Parsed,
    /// Without a parser (W003): its own line breaks, single spaces.
    Unknown,
    /// Quarantined, opaque or nested too deep: as written.
    AsWritten,
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

/// Deepest bracket nesting in a statement: tokens inside one pair of
/// brackets (or one embedded template expression) are at 1.
fn bracket_nesting(stream: &TokenStream, st: &Statement) -> u32 {
    let mut depth = 0u32;
    let mut max = 0;
    for t in &stream.tokens[st.tokens.clone()] {
        match t.kind {
            LPAREN | LBRACKET | LBRACE | TEMPLATE_HEAD => depth += 1,
            RPAREN | RBRACKET | RBRACE | TEMPLATE_TAIL => depth = depth.saturating_sub(1),
            _ => max = max.max(depth),
        }
    }
    max
}

/// A laid-out statement before cross-statement alignment.
struct Entry {
    depth: u32,
    /// The line break before the statement (none for the first statement
    /// and for one glued to the previous statement).
    start: Doc,
    glued: bool,
    body: Stmt,
}

impl Entry {
    fn key(&self) -> Option<&AlignKey> {
        match &self.body {
            Stmt::Row(r) if !self.glued => Some(&r.key),
            _ => None,
        }
    }
}

/// ABAP-ALIGN-004 to ABAP-ALIGN-007: consecutive statements with the same
/// alignment key at the same level are aligned as one group. Blank lines and
/// comment lines between them do not end the group (they are trivia of the
/// statements' first tokens); any other statement does.
fn align_statements(entries: Vec<Entry>) -> Vec<Doc> {
    let mut docs = Vec::with_capacity(entries.len());
    let mut entries = entries.into_iter().peekable();
    while let Some(e) = entries.next() {
        let mut group: Vec<Row> = Vec::new();
        while let Some(next) = entries.peek()
            && e.key().is_some()
            && next.depth == e.depth
            && next.key() == e.key()
        {
            let Some(Entry {
                body: Stmt::Row(r), ..
            }) = entries.next()
            else {
                unreachable!()
            };
            group.push(r);
        }
        let Entry {
            depth, start, body, ..
        } = e;
        let body = match body {
            Stmt::Row(r) if !group.is_empty() => {
                let rows = std::iter::once(r).chain(group).map(Row::aligned).collect();
                b::align_lines(rows)
            }
            other => other.into_doc(),
        };
        docs.push(nest(depth, b::concat(vec![start, body])));
    }
    docs
}

pub fn layout(
    text: &str,
    stream: &TokenStream,
    statements: &[Statement],
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
    explain: bool,
    style: Style,
) -> Laid {
    let mut commented = vec![false; stream.tokens.len() + 1];
    let mut eol_commented = vec![false; stream.tokens.len() + 1];
    for t in &stream.trivia {
        if t.is_comment() {
            match t.position {
                TriviaPosition::Leading => commented[t.anchor.0 as usize] = true,
                TriviaPosition::Trailing => eol_commented[t.anchor.0 as usize] = true,
            }
        }
    }
    let mut ctx = Ctx {
        text,
        style,
        stream,
        commented: &commented,
        eol_commented: &eol_commented,
        keywords: Vec::new(),
        explain,
        rule_spans: Vec::new(),
    };
    let mut entries: Vec<Entry> = Vec::with_capacity(statements.len());
    let mut parsed_ranges = Vec::new();
    let mut diagnostics = Vec::new();
    let mut unknown = 0;
    let mut depth_limited = 0;
    let mut bodies = Vec::with_capacity(statements.len());
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
        // Nested too deep: left as written, so neither the parsers' recursion
        // nor the layout grows with the nesting.
        let too_deep = st.depth + bracket_nesting(stream, st) > MAX_NESTING;
        if too_deep {
            if depth_limited == 0 {
                let range =
                    stream.tokens[first.0 as usize].start..stream.tokens[last.0 as usize].end;
                diagnostics.push(Diagnostic::new(
                    Code::W004,
                    format!(
                        "Nesting deeper than {MAX_NESTING} levels; the deeper statements are left as written"
                    ),
                    range,
                ));
            }
            depth_limited += 1;
        }
        let body: Stmt = if opaque.contains(&id)
            || too_deep
            || (glued_start && stream.tokens[first.0 as usize].kind != crate::kinds::OPAQUE)
        {
            bodies.push(Body::AsWritten);
            Stmt::Doc(b::opaque(first, last, blank))
        } else if quarantine.contains(&id) {
            bodies.push(Body::AsWritten);
            Stmt::Doc(b::verbatim(first, last, blank))
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
                    parsed_ranges.push(st.tokens.clone());
                    bodies.push(Body::Parsed);
                    d
                }
                None => {
                    ctx.reset(saved);
                    mark_ctor(&mut ctx);
                    unknown += 1;
                    bodies.push(Body::Unknown);
                    let t = &stream.tokens[first.0 as usize];
                    diagnostics.push(Diagnostic::new(
                        Code::W003,
                        "Statement not recognized; formatted generically",
                        t.start..stream.tokens[last.0 as usize].end,
                    ));
                    Stmt::Doc(b::unknown(first, last, blank))
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
        entries.push(Entry {
            depth: st.depth.min(MAX_NESTING),
            start,
            glued,
            body,
        });
        prev_role = st.role;
    }
    Laid {
        doc: b::concat(align_statements(entries)),
        diagnostics,
        unknown,
        keywords: ctx.keywords,
        parsed: parsed_ranges,
        rule_spans: ctx.rule_spans,
        depth_limited,
        bodies,
    }
}

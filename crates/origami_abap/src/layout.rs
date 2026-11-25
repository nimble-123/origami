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
use crate::stmt::{AlignKey, Ctx, DeclCols, P, Row, Stmt, decl_columns};
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
    /// First tokens of statements directly after a block opener (or at the
    /// start): a blank line between their comment lines and them is kept in
    /// the aligned style (ABAP-BLANK-001).
    pub blank_after_comments: BTreeSet<usize>,
    /// ABAP-BLANK-002: blank lines added in front of these tokens.
    pub forced_blanks: std::collections::BTreeMap<usize, u8>,
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
            Stmt::Row(r) if !group.is_empty() && matches!(r.key, AlignKey::Decl(_)) => {
                let all: Vec<Row> = std::iter::once(r).chain(group).collect();
                let cols: Vec<&DeclCols> = all.iter().filter_map(|r| r.decl.as_deref()).collect();
                let (value_col, ro_col) = decl_columns(&cols);
                let rows = all
                    .into_iter()
                    .map(|r| match r.decl {
                        Some(d) => (*d).row(value_col, ro_col),
                        None => r.aligned(),
                    })
                    .collect();
                b::align_lines(rows)
            }
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
        multi_line: false,
    };
    let mut entries: Vec<Entry> = Vec::with_capacity(statements.len());
    let mut parsed_ranges = Vec::new();
    let mut diagnostics = Vec::new();
    let mut unknown = 0;
    let mut depth_limited = 0;
    let mut bodies = Vec::with_capacity(statements.len());
    let mut prev_role = None;
    let mut blank_after_comments = BTreeSet::new();
    let mut multi = Vec::with_capacity(statements.len());
    for (id, st) in statements.iter().enumerate() {
        ctx.multi_line = false;
        let id = id as UnitId;
        let first = TokenIndex(st.tokens.start as u32);
        let last = TokenIndex(st.tokens.end as u32 - 1);
        // One blank line is kept, except directly after an opener and
        // directly before a closer.
        let blank = id > 0 && !opens(prev_role) && !closes(st.role);
        if style == Style::Aligned && !blank && !closes(st.role) {
            blank_after_comments.insert(first.0 as usize);
        }
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
        let parsed_multi = ctx.multi_line && bodies.last() == Some(&Body::Parsed);
        let raw_multi = bodies.last() != Some(&Body::Parsed)
            && text[stream.tokens[first.0 as usize].start as usize
                ..stream.tokens[last.0 as usize].end as usize]
                .contains('\n');
        multi.push(parsed_multi || raw_multi);
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
    let forced_blanks = if style == Style::Aligned {
        forced_blanks(text, stream, statements, &multi)
    } else {
        Default::default()
    };
    Laid {
        doc: b::concat(align_statements(entries)),
        diagnostics,
        unknown,
        keywords: ctx.keywords,
        parsed: parsed_ranges,
        rule_spans: ctx.rule_spans,
        depth_limited,
        bodies,
        blank_after_comments,
        forced_blanks,
    }
}

/// Declaration keywords of class definitions and interfaces
/// (ABAP-BLANK-002).
const DEF_DECLS: &[&str] = &[
    "INTERFACES",
    "ALIASES",
    "TYPES",
    "CONSTANTS",
    "DATA",
    "CLASS-DATA",
    "METHODS",
    "CLASS-METHODS",
    "EVENTS",
    "CLASS-EVENTS",
];

/// Declarations at the start of a procedure (ABAP-BLANK-002).
const PROC_DECLS: &[&str] = &["DATA", "TYPES", "CONSTANTS", "STATICS", "FIELD-SYMBOLS"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Block {
    ClassDef,
    ClassImpl,
    Interface,
    /// `METHOD`, `FORM`, `FUNCTION`: `true` while only declarations have
    /// been seen at its start, `false` once they are over.
    Proc(bool, bool),
    Other,
}

/// ABAP-BLANK-002 (aligned style): blank lines to add, as the number of
/// blank lines in front of a statement's first token (and its comments).
fn forced_blanks(
    text: &str,
    stream: &TokenStream,
    statements: &[Statement],
    multi: &[bool],
) -> std::collections::BTreeMap<usize, u8> {
    let mut out = std::collections::BTreeMap::new();
    let word = |st: &Statement, k: usize| -> String {
        stream
            .tokens
            .get(st.tokens.start + k)
            .filter(|_| st.tokens.start + k < st.tokens.end)
            .map(|t| t.text(text).to_ascii_uppercase())
            .unwrap_or_default()
    };
    // A declaration statement: its keyword, unless it is an inline
    // declaration (`DATA(x) = …`).
    let decl_kw = |st: &Statement, list: &[&str]| -> Option<String> {
        let w = word(st, 0);
        let next = stream.tokens.get(st.tokens.start + 1)?;
        let inline = next.glued && next.kind != crate::kinds::COLON;
        (list.contains(&w.as_str()) && !inline).then_some(w)
    };
    let structure = |st: &Statement| -> i32 {
        let (a, b) = (word(st, 1), word(st, 2));
        let (a, b) = if a == ":" {
            (word(st, 2), word(st, 3))
        } else {
            (a, b)
        };
        match (a.as_str(), b.as_str()) {
            ("BEGIN", "OF") => 1,
            ("END", "OF") => -1,
            _ => 0,
        }
    };
    let mut stack: Vec<Block> = Vec::new();
    // Class definition / interface: the previous declaration of this
    // section (keyword, spans several lines), and the open structure depth.
    let mut last: Option<(String, bool)> = None;
    let mut in_structure = 0i32;
    let mut prev: Option<&Statement> = None;
    for (n, st) in statements.iter().enumerate() {
        let first = st.tokens.start;
        let w0 = word(st, 0);
        let prev_w0 = prev.map(|p| word(p, 0)).unwrap_or_default();
        let mut force = 0u8;
        match stack.last().copied() {
            None if matches!(w0.as_str(), "CLASS" | "INTERFACE")
                && matches!(prev_w0.as_str(), "ENDCLASS" | "ENDINTERFACE") =>
            {
                force = 2;
            }
            Some(Block::ClassImpl) if w0 == "METHOD" && prev_w0 == "ENDMETHOD" => force = 1,
            Some(Block::ClassDef | Block::Interface) => {
                let section = word(st, 1) == "SECTION"
                    && matches!(w0.as_str(), "PUBLIC" | "PROTECTED" | "PRIVATE");
                if section {
                    if prev.is_some_and(|p| word(p, 0) != "CLASS") {
                        force = 1;
                    }
                    last = None;
                    in_structure = 0;
                } else if in_structure > 0 {
                    in_structure += structure(st);
                    if in_structure <= 0 {
                        in_structure = 0;
                        last = Some(("TYPES".into(), true));
                    }
                } else if let Some(kw) = decl_kw(st, DEF_DECLS) {
                    let s = structure(st);
                    let is_multi = multi[n] || s > 0;
                    if let Some((lk, lm)) = &last
                        && (lk != &kw || *lm || is_multi)
                    {
                        force = 1;
                    }
                    if s > 0 && word(st, 1) != ":" {
                        in_structure = s;
                        last = Some((kw, true));
                    } else {
                        last = Some((kw, is_multi));
                    }
                } else if !matches!(st.role, Some(Role::Closer)) {
                    last = None;
                }
            }
            Some(Block::Proc(true, seen)) => {
                if decl_kw(st, PROC_DECLS).is_some() {
                    if let Some(Block::Proc(_, s)) = stack.last_mut() {
                        *s = true;
                    }
                } else {
                    if seen && !matches!(st.role, Some(Role::Closer)) {
                        force = 1;
                    }
                    if let Some(Block::Proc(start, _)) = stack.last_mut() {
                        *start = false;
                    }
                }
            }
            _ => {}
        }
        if force > 0 {
            out.insert(first, force);
        }
        match st.role {
            Some(Role::Opener(_)) => {
                let w = |k| word(st, k);
                let has = |x: &str| (1..st.tokens.len()).any(|k| w(k) == x);
                stack.push(match w0.as_str() {
                    "CLASS" if has("DEFINITION") => {
                        last = None;
                        Block::ClassDef
                    }
                    "CLASS" => Block::ClassImpl,
                    "INTERFACE" => {
                        last = None;
                        Block::Interface
                    }
                    "METHOD" | "FORM" | "FUNCTION" => Block::Proc(true, false),
                    _ => Block::Other,
                });
            }
            Some(Role::Closer) => {
                stack.pop();
            }
            _ => {}
        }
        prev = Some(st);
    }
    out
}

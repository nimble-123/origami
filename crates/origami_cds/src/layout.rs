//! Structural layout for the CDS family (STYLE CDS-*): one annotation or
//! element per line, indentation by `{ }` blocks, short inline blocks kept on
//! one line, non-key select-list elements aligned under the names of key
//! elements. Letter case is never changed. Inside a unit, the author's line
//! breaks are kept and re-indented, and runs of spaces shrink to one. Units
//! nested deeper than `MAX_NESTING` are printed byte for byte (W004). In a DDLS
//! query, `where`, `group by`, `having`, `union`, `except` and `intersect`
//! each start a unit, so a line break before them (CDS-CLAUSE-001) is always
//! there and their continuation lines keep their position relative to them.

use crate::kinds::*;
use origami_layout::{Doc, builder as b};
use origami_syntax::{
    Code, Diagnostic, Language, LineIndex, MAX_NESTING, RuleSpan, TokenIndex, TokenStream, UnitId,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Annotation,
    Open,
    Close,
    Element,
}

#[derive(Debug, Clone)]
pub struct CdsUnit {
    pub first: usize,
    pub last: usize,
    kind: Kind,
    depth: u32,
    /// Extra indent (in indent steps) for non-key elements in a block with
    /// key elements.
    extra: u32,
}

/// Whether token `k` starts a clause of a DDLS query after the select list
/// (CDS-CLAUSE-001): `where`, `group by`, `having`, `union`, `except` or
/// `intersect`, not glued to the token before.
fn is_clause_start(text: &str, toks: &[origami_syntax::Token], k: usize) -> bool {
    let word = |k: usize| {
        toks.get(k)
            .filter(|t| t.kind == IDENT && !t.glued)
            .map(|t| t.text(text))
    };
    let Some(w) = word(k) else { return false };
    ["where", "having", "union", "except", "intersect"]
        .iter()
        .any(|c| w.eq_ignore_ascii_case(c))
        || (w.eq_ignore_ascii_case("group")
            && word(k + 1).is_some_and(|b| b.eq_ignore_ascii_case("by")))
}

/// The layout of a clause unit that starts with `group` and whose `by` is on
/// a later line: `by` moves up behind `group`, and the continuation lines
/// keep their position relative to `by`. Otherwise the unit as written.
fn clause_doc(text: &str, stream: &TokenStream, u: &CdsUnit, blank: bool) -> Doc {
    let toks = &stream.tokens;
    let (first, last) = (TokenIndex(u.first as u32), TokenIndex(u.last as u32));
    let by = u.first + 1;
    let join = by <= u.last
        && toks[u.first].text(text).eq_ignore_ascii_case("group")
        && toks[by].text(text).eq_ignore_ascii_case("by")
        && {
            let gap = &text[toks[u.first].end as usize..toks[by].start as usize];
            gap.contains('\n') && gap.chars().all(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
        };
    if !join {
        return b::unknown(first, last, blank);
    }
    let group = if blank {
        b::token_keep_blank(first)
    } else {
        b::token(first)
    };
    let by = TokenIndex(by as u32);
    b::concat(vec![
        group,
        b::keep(stream, by),
        b::unknown(by, last, false),
    ])
}

/// Index after the annotation value starting at `k` (after the colon).
fn skip_value(toks: &[origami_syntax::Token], mut k: usize) -> usize {
    if k >= toks.len() {
        return k;
    }
    match toks[k].kind {
        LBRACE | LBRACKET | LPAREN => {
            let mut depth = 0i32;
            while k < toks.len() {
                match toks[k].kind {
                    LBRACE | LBRACKET | LPAREN => depth += 1,
                    RBRACE | RBRACKET | RPAREN => {
                        depth -= 1;
                        if depth == 0 {
                            return k + 1;
                        }
                    }
                    _ => {}
                }
                k += 1;
            }
            k
        }
        // No value (`@a:` before a closing bracket or separator): the token
        // belongs to the enclosing structure, not to the annotation.
        RBRACE | RBRACKET | RPAREN | COMMA | SEMICOLON => k,
        _ => {
            k += 1;
            // Dotted or glued continuation (`#X`, `a.b`, `'x'`, negative numbers).
            while k < toks.len() && toks[k].glued && matches!(toks[k].kind, DOT | IDENT | NUMBER) {
                k += 1;
            }
            k
        }
    }
}

/// For every `{`, the index of its matching `}` (`None` for other tokens and
/// for a `{` that is never closed).
fn matching_braces(toks: &[origami_syntax::Token]) -> Vec<Option<usize>> {
    let mut close = vec![None; toks.len()];
    let mut open = Vec::new();
    for (k, t) in toks.iter().enumerate() {
        match t.kind {
            LBRACE => open.push(k),
            RBRACE => {
                if let Some(o) = open.pop() {
                    close[o] = Some(k);
                }
            }
            _ => {}
        }
    }
    close
}

/// Nesting of every token in brackets of any kind: tokens inside one pair
/// are at 1; a bracket is at the level of the tokens around it.
fn nesting(toks: &[origami_syntax::Token]) -> Vec<u32> {
    let mut depth = 0u32;
    toks.iter()
        .map(|t| match t.kind {
            LBRACE | LPAREN | LBRACKET => {
                depth += 1;
                depth - 1
            }
            RBRACE | RPAREN | RBRACKET => {
                depth = depth.saturating_sub(1);
                depth
            }
            _ => depth,
        })
        .collect()
}

/// Splits the token stream into layout units. A `;` ends a unit in every
/// language; a `,` ends one only inside a DDLS block (select-list elements).
pub fn units(text: &str, stream: &TokenStream, lang: Language) -> Vec<CdsUnit> {
    let toks = &stream.tokens;
    let n = toks.len();
    let lines = LineIndex::new(text);
    let line_of = |k: usize| lines.line_of(toks[k].start);
    let close_of = matching_braces(toks);
    let mut out = Vec::new();
    let mut depth = 0u32;
    let mut i = 0;
    while i < n {
        let t = &toks[i];
        match t.kind {
            AT | AT_LT => {
                let mut j = i + 1;
                while j < n && toks[j].glued && matches!(toks[j].kind, IDENT | DOT | DOLLAR_IDENT) {
                    j += 1;
                }
                if j < n && toks[j].kind == COLON {
                    j = skip_value(toks, j + 1);
                }
                // A following comma or semicolon belongs to the annotation.
                if j < n && matches!(toks[j].kind, COMMA | SEMICOLON) {
                    j += 1;
                }
                out.push(CdsUnit {
                    first: i,
                    last: j - 1,
                    kind: Kind::Annotation,
                    depth,
                    extra: 0,
                });
                i = j;
            }
            LBRACE => {
                out.push(CdsUnit {
                    first: i,
                    last: i,
                    kind: Kind::Open,
                    depth,
                    extra: 0,
                });
                depth += 1;
                i += 1;
            }
            RBRACE => {
                depth = depth.saturating_sub(1);
                let mut j = i + 1;
                if j < n && matches!(toks[j].kind, COMMA | SEMICOLON) {
                    j += 1;
                }
                out.push(CdsUnit {
                    first: i,
                    last: j - 1,
                    kind: Kind::Close,
                    depth,
                    extra: 0,
                });
                i = j;
            }
            _ => {
                let mut j = i;
                let mut nest = 0i32;
                while j < n {
                    let k = toks[j].kind;
                    if nest == 0 {
                        if matches!(k, RBRACE) || (matches!(k, AT | AT_LT) && j > i) {
                            break;
                        }
                        // A query clause after the select list starts a unit.
                        if lang == Language::Ddls
                            && depth == 0
                            && j > i
                            && is_clause_start(text, toks, j)
                        {
                            break;
                        }
                        if k == LBRACE {
                            // A short block that the author wrote on one line,
                            // right after the previous token, stays part of
                            // this unit. (Both conditions survive formatting,
                            // so a second run makes the same choice.)
                            match close_of[j] {
                                Some(close)
                                    if line_of(j - 1) == line_of(j)
                                        && line_of(j) == line_of(close) =>
                                {
                                    // The block ends the unit
                                    // (`use association _x { with draft; }`).
                                    j = close + 1;
                                    if j < n && matches!(toks[j].kind, COMMA | SEMICOLON) {
                                        j += 1;
                                    }
                                    break;
                                }
                                _ => break,
                            }
                        }
                    }
                    match k {
                        LPAREN | LBRACKET => nest += 1,
                        RPAREN | RBRACKET => nest -= 1,
                        _ => {}
                    }
                    j += 1;
                    let element_comma = k == COMMA && lang == Language::Ddls && depth > 0;
                    if nest == 0 && (k == SEMICOLON || element_comma) {
                        break;
                    }
                }
                if j == i {
                    j = i + 1;
                }
                out.push(CdsUnit {
                    first: i,
                    last: j - 1,
                    kind: Kind::Element,
                    depth,
                    extra: 0,
                });
                i = j;
            }
        }
    }
    mark_key_blocks(text, stream, &mut out);
    out
}

/// In a block whose elements include `key …` elements, the other elements
/// (and their annotations) are indented by `key ` so names line up.
fn mark_key_blocks(text: &str, stream: &TokenStream, units: &mut [CdsUnit]) {
    let is_key = |u: &CdsUnit| {
        u.kind == Kind::Element
            && stream.tokens[u.first].kind == IDENT
            && stream.tokens[u.first]
                .text(text)
                .eq_ignore_ascii_case("key")
    };
    // The members of every block (the units one level inside it), found in
    // one pass, so nested blocks cost no repeated scans.
    let mut open: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut blocks = Vec::new();
    for m in 0..units.len() {
        if units[m].kind == Kind::Close
            && open
                .last()
                .is_some_and(|(k, _)| units[*k].depth == units[m].depth)
        {
            blocks.extend(open.pop().map(|(_, members)| members));
        }
        if let Some((k, members)) = open.last_mut()
            && units[m].depth == units[*k].depth + 1
        {
            members.push(m);
        }
        if units[m].kind == Kind::Open {
            open.push((m, Vec::new()));
        }
    }
    blocks.extend(open.into_iter().map(|(_, members)| members));
    for members in blocks {
        if members.iter().any(|&m| is_key(&units[m])) {
            // Walk backwards so annotations take the indent of their element.
            let mut next_is_key = true;
            for &m in members.iter().rev() {
                match units[m].kind {
                    Kind::Element => {
                        next_is_key = is_key(&units[m]);
                        if !next_is_key {
                            units[m].extra = 2;
                        }
                    }
                    Kind::Annotation if !next_is_key => units[m].extra = 2,
                    _ => {}
                }
            }
        }
    }
}

pub struct Laid {
    pub doc: Doc,
    pub rule_spans: Vec<RuleSpan>,
    pub diagnostics: Vec<Diagnostic>,
    /// Units left as written because they nest too deep (W004).
    pub depth_limited: u32,
}

pub fn layout(
    text: &str,
    stream: &TokenStream,
    lang: Language,
    units: &[CdsUnit],
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
    explain: bool,
) -> Laid {
    let mut docs = Vec::with_capacity(units.len());
    let mut spans = Vec::new();
    let mut diagnostics = Vec::new();
    let mut depth_limited = 0;
    let nest = nesting(&stream.tokens);
    for (id, u) in units.iter().enumerate() {
        let first = TokenIndex(u.first as u32);
        let last = TokenIndex(u.last as u32);
        let prev = id.checked_sub(1).map(|p| units[p].kind);
        let blank = id > 0 && prev != Some(Kind::Open) && u.kind != Kind::Close;
        let uid = id as UnitId;
        let clause = lang == Language::Ddls
            && u.kind == Kind::Element
            && u.depth == 0
            && is_clause_start(text, &stream.tokens, u.first);
        // Nested too deep: left as written, so neither the grammar's
        // recursion nor the layout grows with the nesting.
        let too_deep = nest[u.first..=u.last].iter().any(|&d| d > MAX_NESTING);
        if too_deep {
            if depth_limited == 0 {
                diagnostics.push(Diagnostic::new(
                    Code::W004,
                    format!(
                        "Nesting deeper than {MAX_NESTING} levels; the deeper elements are left as written"
                    ),
                    stream.tokens[u.first].start..stream.tokens[u.last].end,
                ));
            }
            depth_limited += 1;
        }
        // A unit glued to the one before starts no line. If its layout
        // joined its lines, a block written over several lines could print
        // on one, and the next run would keep it in the unit before
        // (`units`, fuzz/found/cds-glued-block.bin). Such a unit inside a
        // block keeps its line breaks.
        let glued_inside = u.depth > 0
            && matches!(u.kind, Kind::Element | Kind::Annotation)
            && stream.tokens[u.first].glued;
        let body = if opaque.contains(&uid) || too_deep {
            b::opaque(first, last, blank)
        } else if quarantine.contains(&uid) {
            b::verbatim(first, last, blank)
        } else if glued_inside {
            b::unknown(first, last, blank)
        } else {
            let g = crate::grammar::G::new(text, stream, u.first, blank, explain);
            // The parsed layout and the rule for the whole unit (a case
            // expression records its own spans).
            let with = |d: Option<Doc>, rule| d.map(|d| (d, rule));
            let parsed = match u.kind {
                Kind::Annotation => with(g.annotation(u.first, u.last), Some("CDS-ANNO-001")),
                Kind::Element => with(g.header(u.first, u.last, lang), Some("CDS-HEAD-001"))
                    .or_else(|| match lang {
                        Language::Ddls if u.depth > 0 => with(g.element(u.first, u.last), None),
                        Language::Dcl => with(g.grant(u.first, u.last), Some("DCL-GRANT-001")),
                        _ => None,
                    }),
                _ => None,
            };
            match parsed {
                Some((d, rule)) => {
                    if let Some(rule) = rule.filter(|_| explain) {
                        spans.push(crate::grammar::G::span(u.first, u.last, rule));
                    }
                    spans.extend(g.spans.into_inner());
                    d
                }
                None if clause => {
                    if explain && id > 0 {
                        spans.push(crate::grammar::G::span(
                            u.first - 1,
                            u.first,
                            "CDS-CLAUSE-001",
                        ));
                    }
                    clause_doc(text, stream, u, blank)
                }
                None => b::unknown(first, last, blank),
            }
        };
        let glued = stream.tokens[u.first].glued;
        let start = if id == 0 || glued {
            b::empty()
        } else {
            b::hard(stream, first)
        };
        let level = if too_deep {
            (u.depth + u.extra).min(MAX_NESTING)
        } else {
            u.depth + u.extra
        };
        docs.push((0..level).fold(b::concat(vec![start, body]), |d, _| b::indent(vec![d])));
    }
    Laid {
        doc: b::concat(docs),
        rule_spans: spans,
        diagnostics,
        depth_limited,
    }
}

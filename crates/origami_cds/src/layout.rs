//! Structural layout for the CDS family (STYLE CDS-*): one annotation or
//! element per line, indentation by `{ }` blocks, short inline blocks kept on
//! one line, non-key select-list elements aligned under the names of key
//! elements. Letter case is never changed. Inside a unit, the author's line
//! breaks are kept and re-indented, and runs of spaces shrink to one.

use crate::kinds::*;
use origami_layout::{Doc, builder as b};
use origami_syntax::{Language, RuleSpan, TokenIndex, TokenStream, UnitId};
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

fn line_of(text: &str, offset: u32) -> usize {
    text[..offset as usize].matches('\n').count()
}

/// Index after the annotation value starting at `k` (after the colon).
fn skip_value(toks: &[origami_syntax::Token], mut k: usize) -> usize {
    if k >= toks.len() {
        return k;
    }
    match toks[k].kind {
        LBRACE | LBRACKET => {
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

/// Matching `}` for the `{` at `open`.
fn matching_brace(toks: &[origami_syntax::Token], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (k, t) in toks.iter().enumerate().skip(open) {
        match t.kind {
            LBRACE => depth += 1,
            RBRACE => {
                depth -= 1;
                if depth == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}

/// Splits the token stream into layout units. A `;` ends a unit in every
/// language; a `,` ends one only inside a DDLS block (select-list elements).
pub fn units(text: &str, stream: &TokenStream, lang: Language) -> Vec<CdsUnit> {
    let toks = &stream.tokens;
    let n = toks.len();
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
                        if k == LBRACE {
                            // A short block that the author wrote on one line,
                            // right after the previous token, stays part of
                            // this unit. (Both conditions survive formatting,
                            // so a second run makes the same choice.)
                            match matching_brace(toks, j) {
                                Some(close)
                                    if line_of(text, toks[j - 1].start)
                                        == line_of(text, toks[j].start)
                                        && line_of(text, toks[j].start)
                                            == line_of(text, toks[close].start) =>
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
    let mut k = 0;
    while k < units.len() {
        if units[k].kind != Kind::Open {
            k += 1;
            continue;
        }
        let inner = units[k].depth + 1;
        let mut end = k + 1;
        while end < units.len()
            && !(units[end].kind == Kind::Close && units[end].depth == units[k].depth)
        {
            end += 1;
        }
        let members: Vec<usize> = (k + 1..end).filter(|&m| units[m].depth == inner).collect();
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
        k += 1;
    }
}

pub fn layout(
    text: &str,
    stream: &TokenStream,
    lang: Language,
    units: &[CdsUnit],
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
    explain: bool,
) -> (Doc, Vec<RuleSpan>) {
    let mut docs = Vec::with_capacity(units.len());
    let mut spans = Vec::new();
    for (id, u) in units.iter().enumerate() {
        let first = TokenIndex(u.first as u32);
        let last = TokenIndex(u.last as u32);
        let prev = id.checked_sub(1).map(|p| units[p].kind);
        let blank = id > 0 && prev != Some(Kind::Open) && u.kind != Kind::Close;
        let uid = id as UnitId;
        let body = if opaque.contains(&uid) {
            b::opaque(first, last, blank)
        } else if quarantine.contains(&uid) {
            b::verbatim(first, last, blank)
        } else {
            let g = crate::grammar::G::new(text, stream, u.first, blank, explain);
            let parsed = match u.kind {
                Kind::Annotation => g.annotation(u.first, u.last),
                Kind::Element => g.header(u.first, u.last, lang).or_else(|| match lang {
                    Language::Ddls if u.depth > 0 => g.element(u.first, u.last),
                    Language::Dcl => g.grant(u.first, u.last),
                    _ => None,
                }),
                _ => None,
            };
            match parsed {
                Some(d) => {
                    spans.extend(g.spans.into_inner());
                    d
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
        let level = u.depth + u.extra;
        docs.push((0..level).fold(b::concat(vec![start, body]), |d, _| b::indent(vec![d])));
    }
    (b::concat(docs), spans)
}

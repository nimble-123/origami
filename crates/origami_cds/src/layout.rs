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
    Code, Diagnostic, Language, LineIndex, MAX_NESTING, RuleSpan, TokenIndex, TokenStream,
    TriviaPosition, UnitId,
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

/// The unit has no comment after its first token.
fn no_comments(stream: &TokenStream, u: &CdsUnit) -> bool {
    let (a, b) = (stream.tokens[u.first].start, stream.tokens[u.last].end);
    !stream
        .trivia
        .iter()
        .any(|t| t.is_comment() && t.start > a && t.end <= b)
}

/// `from..to` printed on one line with single spaces (glued tokens stay
/// glued).
fn flat_tokens(stream: &TokenStream, from: usize, to: usize) -> Vec<Doc> {
    let mut docs = Vec::new();
    for k in from..to {
        if k > from && !stream.tokens[k].glued {
            docs.push(b::sep(stream, TokenIndex(k as u32)));
        }
        docs.push(b::token(TokenIndex(k as u32)));
    }
    docs
}

/// CDS-ALIGN-004: `group by a, b` with two or more fields, one field per
/// line behind `group by`. `None` for other clauses, lists with one field,
/// or units with comments.
fn group_by_doc(text: &str, stream: &TokenStream, u: &CdsUnit, blank: bool) -> Option<Doc> {
    let toks = &stream.tokens;
    let by = u.first + 1;
    if by >= u.last
        || !toks[u.first].text(text).eq_ignore_ascii_case("group")
        || !toks[by].text(text).eq_ignore_ascii_case("by")
        || toks[by].glued
        || !no_comments(stream, u)
    {
        return None;
    }
    // Fields: split at commas on bracket depth 0.
    let mut items: Vec<(usize, usize)> = Vec::new();
    let (mut depth, mut a) = (0i32, by + 1);
    for (k, t) in toks.iter().enumerate().take(u.last + 1).skip(by + 1) {
        match t.kind {
            LPAREN | LBRACKET => depth += 1,
            RPAREN | RBRACKET => depth -= 1,
            COMMA if depth == 0 => {
                items.push((a, k));
                a = k + 1;
            }
            _ => {}
        }
    }
    items.push((a, u.last + 1));
    if items.len() < 2 || items.iter().any(|&(a, e)| a >= e || toks[a].glued) {
        return None;
    }
    let first = TokenIndex(u.first as u32);
    let mut list = Vec::new();
    for (n, &(a, e)) in items.iter().enumerate() {
        if n > 0 {
            list.push(b::hard(stream, TokenIndex(a as u32)));
        }
        list.extend(flat_tokens(stream, a, e));
        // The comma after the field.
        if e <= u.last {
            list.extend(flat_tokens(stream, e, e + 1));
        }
    }
    Some(b::concat(vec![
        if blank {
            b::token_keep_blank(first)
        } else {
            b::token(first)
        },
        b::sep(stream, TokenIndex(by as u32)),
        b::token(TokenIndex(by as u32)),
        b::sep(stream, TokenIndex(by as u32 + 1)),
        b::hang(list),
    ]))
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
    // Tokens followed by a comment on their line: the comment ends the line.
    let mut eol_comment = vec![false; n];
    for t in &stream.trivia {
        if t.is_comment()
            && t.position == TriviaPosition::Trailing
            && let Some(c) = eol_comment.get_mut(t.anchor.0 as usize)
        {
            *c = true;
        }
    }
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
                            // so a second run makes the same choice.) A
                            // comment in it, or in front of it in this unit,
                            // ends a line and can make the unit's layout break
                            // the block, and so can a `case`, which breaks when
                            // it does not fit (CDS-ELEM-001); such a block is
                            // split (fuzz/found/cds-bdef-comment-block.bin,
                            // cds-comment-before-block.bin, cds-case-before-block.bin).
                            match close_of[j] {
                                Some(close)
                                    if line_of(j - 1) == line_of(j)
                                        && line_of(j) == line_of(close)
                                        && !eol_comment[i..close].contains(&true)
                                        && !(i..close).any(|k| {
                                            toks[k].kind == IDENT
                                                && toks[k].text(text).eq_ignore_ascii_case("case")
                                        }) =>
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
    /// CDS-BLANK-002: blank lines added in front of these tokens.
    pub forced_blanks: std::collections::BTreeMap<usize, u8>,
}

// `opts` is one more argument than the entry point had before the styles.
#[allow(clippy::too_many_arguments)]
pub fn layout(
    text: &str,
    stream: &TokenStream,
    lang: Language,
    units: &[CdsUnit],
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
    explain: bool,
    opts: &origami_syntax::LangOptions,
) -> Laid {
    let aligned = opts.style == origami_syntax::Style::Aligned;
    let view = aligned && lang == Language::Ddls && is_view(text, stream, units);
    let alias_pads = if view {
        alias_pads(text, stream, units, opts.indent_width)
    } else {
        Default::default()
    };
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
            let mut g = crate::grammar::G::new(text, stream, u.first, blank, explain);
            g.aligned = view;
            g.width = opts.line_width as usize;
            g.indent = opts.indent_width as usize;
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
                    match group_by_doc(text, stream, u, blank).filter(|_| view) {
                        Some(d) => {
                            if explain {
                                spans.push(crate::grammar::G::span(
                                    u.first,
                                    u.last,
                                    "CDS-ALIGN-004",
                                ));
                            }
                            d
                        }
                        None => clause_doc(text, stream, u, blank),
                    }
                }
                None => match alias_pads.get(&id) {
                    Some(&(at, pad)) => padded_element(stream, u, at, pad, blank),
                    None => b::unknown(first, last, blank),
                },
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
    let forced_blanks = if view {
        view_blanks(text, stream, units)
    } else {
        Default::default()
    };
    Laid {
        doc: b::concat(docs),
        rule_spans: spans,
        diagnostics,
        depth_limited,
        forced_blanks,
    }
}

fn word(text: &str, stream: &TokenStream, k: usize) -> String {
    match stream.tokens.get(k) {
        Some(t) if t.kind == IDENT => t.text(text).to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// A DDLS view definition (`define [root] view [entity] …`): the unit at
/// depth 0 after the annotations starts with `define` and has `view` among
/// its next words.
fn is_view(text: &str, stream: &TokenStream, units: &[CdsUnit]) -> bool {
    units
        .iter()
        .find(|u| u.kind == Kind::Element && u.depth == 0)
        .is_some_and(|u| {
            word(text, stream, u.first) == "define"
                && (u.first + 1..=(u.first + 3).min(u.last))
                    .any(|k| word(text, stream, k) == "view")
        })
}

/// A `case` in the unit has two or more `when` (CDS-ELEM-001 breaks it).
fn case_breaks(text: &str, stream: &TokenStream, u: &CdsUnit) -> bool {
    let mut whens: Vec<u32> = Vec::new();
    for k in u.first..=u.last {
        match word(text, stream, k).as_str() {
            "case" => whens.push(0),
            "when" => {
                if let Some(n) = whens.last_mut() {
                    *n += 1;
                    if *n >= 2 {
                        return true;
                    }
                }
            }
            "end" => {
                whens.pop();
            }
            _ => {}
        }
    }
    false
}

/// The unit's tokens are on one line and have no comment between them.
fn one_line(text: &str, stream: &TokenStream, u: &CdsUnit) -> bool {
    let toks = &stream.tokens;
    let (a, b) = (toks[u.first].start, toks[u.last].end);
    !text[a as usize..b as usize].contains('\n')
        && !stream
            .trivia
            .iter()
            .any(|t| t.is_comment() && t.start > a && t.end <= b)
}

/// Width of tokens `from..to` printed with single spaces.
fn width(text: &str, stream: &TokenStream, from: usize, to: usize) -> usize {
    (from..to)
        .map(|k| {
            let gap = usize::from(k > from && !stream.tokens[k].glued);
            gap + stream.tokens[k].text(text).chars().count()
        })
        .sum()
}

/// CDS-ALIGN-001: for each one-line select-list element with an alias, the
/// index of its `as` and the padding in front of it. Elements without an
/// alias that are a name or a path count for the column.
fn alias_pads(
    text: &str,
    stream: &TokenStream,
    units: &[CdsUnit],
    indent_width: u8,
) -> std::collections::HashMap<usize, (usize, usize)> {
    let mut out = std::collections::HashMap::new();
    let Some(open) = units
        .iter()
        .position(|u| u.kind == Kind::Open && u.depth == 0)
    else {
        return out;
    };
    // (unit id, offset + width up to `as`, `as` index) and widths of plain
    // paths.
    let mut rows: Vec<(usize, usize, usize)> = Vec::new();
    let mut col = 0usize;
    for (id, u) in units.iter().enumerate().skip(open + 1) {
        if u.kind == Kind::Close && u.depth == 0 {
            break;
        }
        // A `case` element has its own layout (CDS-ELEM-001), and an element
        // that starts with `define` or `extend` is laid out as a header,
        // which joins its lines; neither is padded, so neither counts.
        if u.kind != Kind::Element
            || u.depth != 1
            || !one_line(text, stream, u)
            || (u.first..=u.last).any(|k| word(text, stream, k) == "case")
            || matches!(word(text, stream, u.first).as_str(), "define" | "extend")
        {
            continue;
        }
        let offset = u.extra as usize * indent_width as usize;
        let mut last = u.last;
        if stream.tokens[last].kind == COMMA {
            last -= 1;
        }
        // `… as Alias`: `as` at bracket depth 0, followed by the alias only.
        let mut depth = 0i32;
        let mut as_at = None;
        for k in u.first..=last {
            match stream.tokens[k].kind {
                LPAREN | LBRACKET => depth += 1,
                RPAREN | RBRACKET => depth -= 1,
                _ => {}
            }
            if depth == 0 && k > u.first && word(text, stream, k) == "as" && !stream.tokens[k].glued
            {
                as_at = Some(k);
            }
        }
        match as_at {
            Some(a) if a + 1 == last => {
                let w = offset + width(text, stream, u.first, a);
                col = col.max(w);
                rows.push((id, w, a));
            }
            Some(_) => {}
            None => {
                // A name or a path (`t.customer_id`, `key travel_id`).
                let plain = (u.first..=last).all(|k| {
                    let t = &stream.tokens[k];
                    t.kind == IDENT || (t.kind == DOT && t.glued)
                });
                if plain {
                    col = col.max(offset + width(text, stream, u.first, last + 1));
                }
            }
        }
    }
    if rows.len() < 2 || col > 120 {
        return out;
    }
    for (id, w, a) in rows {
        out.insert(id, (a, col - w));
    }
    out
}

/// An element printed with `pad` extra spaces in front of its `as` (token
/// `at`); otherwise as `b::unknown` prints a one-line unit.
fn padded_element(stream: &TokenStream, u: &CdsUnit, at: usize, pad: usize, blank: bool) -> Doc {
    let mut docs = Vec::new();
    for k in u.first..=u.last {
        let i = TokenIndex(k as u32);
        if k > u.first {
            if k == at {
                docs.push(b::spaces(pad + 1));
            } else if !stream.tokens[k].glued {
                docs.push(b::sep(stream, i));
            }
        }
        docs.push(if k == u.first && blank {
            b::token_keep_blank(i)
        } else {
            b::token(i)
        });
    }
    b::concat(docs)
}

/// CDS-BLANK-002: blank lines between the sections of a view and in its
/// select list.
fn view_blanks(
    text: &str,
    stream: &TokenStream,
    units: &[CdsUnit],
) -> std::collections::BTreeMap<usize, u8> {
    let mut out = std::collections::BTreeMap::new();
    let Some(h) = units
        .iter()
        .position(|u| u.kind == Kind::Element && u.depth == 0)
    else {
        return out;
    };
    let head = &units[h];
    if h > 0 {
        out.insert(head.first, 1);
    }
    // Header clauses (as `G::header` finds them, at bracket depth 0).
    let mut depth = 0i32;
    let (mut params, mut select, mut join, mut assoc) = (false, None, None, None);
    for k in head.first + 1..=head.last {
        match stream.tokens[k].kind {
            LPAREN | LBRACKET | LBRACE => depth += 1,
            RPAREN | RBRACKET | RBRACE => depth -= 1,
            _ => {}
        }
        if depth != 0 || stream.tokens[k].glued {
            continue;
        }
        let w = word(text, stream, k);
        let next = word(text, stream, k + 1);
        let prev = word(text, stream, k - 1);
        let joined = matches!(
            prev.as_str(),
            "inner" | "left" | "right" | "cross" | "outer"
        );
        if w == "with" && next == "parameters" {
            params = true;
        } else if w == "as" && matches!(next.as_str(), "select" | "projection") {
            select.get_or_insert(k);
        } else if (matches!(w.as_str(), "inner" | "left" | "right" | "cross") || w == "join")
            && !joined
            && select.is_some()
        {
            join.get_or_insert(k);
        } else if matches!(w.as_str(), "association" | "composition") && select.is_some() {
            assoc.get_or_insert(k);
        }
    }
    if params && let Some(s) = select {
        out.insert(s, 1);
    }
    for k in [join, assoc].into_iter().flatten() {
        out.insert(k, 1);
    }
    // The select list: `{` and the unit after `}`.
    let Some(open) = units
        .iter()
        .position(|u| u.kind == Kind::Open && u.depth == 0)
    else {
        return out;
    };
    out.insert(units[open].first, 1);
    let close = units
        .iter()
        .enumerate()
        .skip(open + 1)
        .find(|(_, u)| u.kind == Kind::Close && u.depth == 0)
        .map(|(i, _)| i);
    if let Some(c) = close
        && let Some(after) = units.get(c + 1)
    {
        out.insert(after.first, 1);
    }
    // Elements with their annotations: (first token, key, association,
    // spans several lines).
    let aliases: Vec<String> = {
        let mut v = Vec::new();
        for k in head.first..head.last {
            if word(text, stream, k) == "as" && !word(text, stream, k.saturating_sub(1)).is_empty()
            {
                v.push(word(text, stream, k + 1));
            }
        }
        v
    };
    let end = close.unwrap_or(units.len());
    let mut groups: Vec<(usize, bool, bool, bool)> = Vec::new();
    let mut annotated: Option<usize> = None;
    for u in &units[open + 1..end] {
        if u.depth != 1 {
            continue;
        }
        match u.kind {
            Kind::Annotation => {
                annotated.get_or_insert(u.first);
            }
            Kind::Element => {
                let w0 = word(text, stream, u.first);
                let w0_raw = w0.clone();
                let key = w0 == "key";
                let name = if key {
                    word(text, stream, u.first + 1)
                } else {
                    w0
                };
                let assoc = name.starts_with('_')
                    && aliases.contains(&name)
                    && stream
                        .tokens
                        .get(u.first + 1 + usize::from(key))
                        .is_none_or(|t| t.kind != DOT);
                // A `case` element is printed by its rule (CDS-ELEM-001: two
                // or more `when` always break); others keep their lines.
                let has_case = (u.first..=u.last).any(|k| word(text, stream, k) == "case");
                // An element that starts with `define` or `extend` (broken
                // code) is laid out as a header, which may break or join its
                // lines; it never counts as spanning several lines.
                let header = matches!(w0_raw.as_str(), "define" | "extend");
                let multi = annotated.is_some()
                    || if has_case {
                        case_breaks(text, stream, u)
                    } else {
                        !header && !one_line(text, stream, u)
                    };
                groups.push((annotated.take().unwrap_or(u.first), key, assoc, multi));
            }
            _ => {}
        }
    }
    for pair in groups.windows(2) {
        let (p, c) = (pair[0], pair[1]);
        if (p.1 && !c.1) || (c.2 && !p.2) || p.3 || c.3 {
            out.insert(c.0, 1);
        }
    }
    out
}

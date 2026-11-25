//! Grammar-based layout for CDS annotations (CDS-ANNO-001) and for the headers
//! of DDLS views and BDEF behavior definitions (CDS-HEAD-001). Anything these
//! parsers do not fully understand returns `None`, and the unit keeps the
//! structural layout (the author's line breaks, re-indented).

use crate::kinds::*;
use origami_layout::{Doc, builder as b};
use origami_syntax::{Language, RuleId, RuleSpan, TokenIndex, TokenStream};
use std::cell::RefCell;

/// An alignable association: indices of `as` and `on`, widths up to `as`,
/// from `as` to `on`, and from `on` to the end (CDS-ALIGN-001).
type AssocRow = (usize, usize, usize, usize, usize);

/// A name list: `(`, `)` and each name with its comma (CDS-ALIGN-004).
type NameList = (usize, usize, Vec<(usize, usize)>);

pub struct G<'a> {
    pub text: &'a str,
    pub stream: &'a TokenStream,
    pub first: usize,
    pub blank: bool,
    /// Record rule spans (explanations requested).
    pub explain: bool,
    /// Spans of the unit being laid out. A helper collects its spans locally
    /// and appends them here only when it succeeds.
    pub spans: RefCell<Vec<RuleSpan>>,
    /// Aligned style: associations align `as` and `on` (CDS-ALIGN-001).
    pub aligned: bool,
    /// Line width and indent width, for the associations' fit.
    pub width: usize,
    pub indent: usize,
}

impl G<'_> {
    pub fn new<'a>(
        text: &'a str,
        stream: &'a TokenStream,
        first: usize,
        blank: bool,
        explain: bool,
    ) -> G<'a> {
        G {
            text,
            stream,
            first,
            blank,
            explain,
            spans: RefCell::new(Vec::new()),
            aligned: false,
            width: 120,
            indent: 2,
        }
    }

    /// Appends `spans` (collected by a helper that succeeded).
    fn keep_spans(&self, spans: Vec<RuleSpan>) {
        if self.explain {
            self.spans.borrow_mut().extend(spans);
        }
    }

    /// A span for tokens `first..=last` laid out by `rule`.
    pub fn span(first: usize, last: usize, rule: RuleId) -> RuleSpan {
        RuleSpan {
            first: TokenIndex(first as u32),
            last: TokenIndex(last as u32),
            rule,
        }
    }

    fn kind(&self, i: usize) -> u16 {
        self.stream
            .tokens
            .get(i)
            .map(|t| t.kind)
            .unwrap_or(u16::MAX)
    }

    fn glued(&self, i: usize) -> bool {
        self.stream.tokens.get(i).is_some_and(|t| t.glued)
    }

    fn word(&self, i: usize) -> String {
        match self.stream.tokens.get(i) {
            Some(t) if t.kind == IDENT => t.text(self.text).to_ascii_lowercase(),
            _ => String::new(),
        }
    }

    fn t(&self, i: usize) -> Doc {
        if i == self.first && self.blank {
            b::token_keep_blank(TokenIndex(i as u32))
        } else {
            b::token(TokenIndex(i as u32))
        }
    }

    fn sp(&self, i: usize) -> Doc {
        b::keep(self.stream, TokenIndex(i as u32))
    }

    fn ln(&self, i: usize) -> Doc {
        if self.glued(i) {
            b::empty()
        } else {
            b::soft(self.stream, TokenIndex(i as u32))
        }
    }

    fn hl(&self, i: usize) -> Doc {
        if self.glued(i) {
            b::empty()
        } else {
            b::hard(self.stream, TokenIndex(i as u32))
        }
    }

    fn matching(&self, open: usize, last: usize) -> Option<usize> {
        let mut depth = 0i32;
        for j in open..=last {
            match self.kind(j) {
                LBRACE | LBRACKET | LPAREN => depth += 1,
                RBRACE | RBRACKET | RPAREN => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(j);
                    }
                }
                _ => {}
            }
        }
        None
    }

    // ---- annotations ---------------------------------------------------

    /// `@name.path[: value][,|;]` covering exactly `first..=last`.
    pub fn annotation(&self, first: usize, last: usize) -> Option<Doc> {
        if !matches!(self.kind(first), AT | AT_LT) {
            return None;
        }
        let mut docs = vec![self.t(first)];
        let mut j = first + 1;
        while j <= last && self.glued(j) && matches!(self.kind(j), IDENT | DOT | DOLLAR_IDENT) {
            docs.push(self.t(j));
            j += 1;
        }
        if j <= last && self.kind(j) == COLON {
            docs.push(self.sp(j));
            docs.push(self.t(j));
            docs.push(self.sp(j + 1));
            let (value, next) = self.value(j + 1, last)?;
            docs.push(value);
            j = next;
        }
        if j <= last && matches!(self.kind(j), COMMA | SEMICOLON) {
            docs.push(self.sp(j));
            docs.push(self.t(j));
            j += 1;
        }
        (j == last + 1).then(|| b::concat(docs))
    }

    /// A value starting at `j`; returns its doc and the index after it.
    fn value(&self, j: usize, last: usize) -> Option<(Doc, usize)> {
        if j > last {
            return None;
        }
        match self.kind(j) {
            LBRACE | LBRACKET => self.container(j, last),
            _ => {
                let mut k = j;
                let mut docs = Vec::new();
                while k <= last
                    && !matches!(
                        self.kind(k),
                        COMMA | RBRACE | RBRACKET | SEMICOLON | LBRACE | LBRACKET
                    )
                {
                    if k > j {
                        docs.push(self.sp(k));
                    }
                    docs.push(self.t(k));
                    k += 1;
                }
                (k > j).then(|| (b::concat(docs), k))
            }
        }
    }

    /// `{ key: value, … }` or `[ value, … ]`: flat if it fits, otherwise one
    /// item per line, indented one level, with the closing bracket on its own
    /// line (where the brackets are not glued to their content).
    fn container(&self, open: usize, last: usize) -> Option<(Doc, usize)> {
        let close = self.matching(open, last)?;
        let object = self.kind(open) == LBRACE;
        if close == open + 1 {
            return Some((
                b::concat(vec![self.t(open), self.sp(close), self.t(close)]),
                close + 1,
            ));
        }
        let mut items: Vec<Doc> = Vec::new();
        let mut k = open + 1;
        while k < close {
            let item_first = k;
            let mut docs = Vec::new();
            if object {
                // Key path up to the colon.
                while k < close && self.kind(k) != COLON {
                    if !matches!(self.kind(k), IDENT | DOT | DOLLAR_IDENT) {
                        return None;
                    }
                    if k > item_first {
                        docs.push(self.sp(k));
                    }
                    docs.push(self.t(k));
                    k += 1;
                }
                // A key is required (`{ : x }` is not an annotation value).
                if k >= close || k == item_first {
                    return None;
                }
                docs.push(self.sp(k));
                docs.push(self.t(k));
                docs.push(self.sp(k + 1));
                k += 1;
            }
            let (v, next) = self.value(k, close - 1)?;
            docs.push(v);
            k = next;
            if k < close && self.kind(k) == COMMA {
                docs.push(self.sp(k));
                docs.push(self.t(k));
                k += 1;
            } else if k < close {
                return None;
            }
            let sep = self.ln(item_first);
            items.push(b::concat(vec![sep, b::concat(docs)]));
        }
        let doc = b::group(vec![
            self.t(open),
            b::indent(items),
            self.ln(close),
            self.t(close),
        ]);
        Some((doc, close + 1))
    }

    // ---- headers -------------------------------------------------------

    /// Header of a DDLS definition (`define … view entity …` up to `{`) or a
    /// BDEF `define behavior for …` header. Each clause starts a line at one
    /// indent level; view parameters go one level deeper; long `on`
    /// conditions break before `and` / `or`.
    pub fn header(&self, first: usize, last: usize, lang: Language) -> Option<Doc> {
        let w0 = self.word(first);
        if w0 != "define" && w0 != "extend" {
            return None;
        }
        let bdef = lang == Language::Bdef;
        if bdef && self.word(first + 1) != "behavior" {
            return None;
        }
        if !bdef && lang != Language::Ddls {
            return None;
        }
        // Clause starts at bracket depth 0.
        let mut starts = Vec::new();
        let mut depth = 0i32;
        let mut in_params = false;
        let mut params = Vec::new();
        for k in first + 1..=last {
            match self.kind(k) {
                LPAREN | LBRACKET | LBRACE => depth += 1,
                RPAREN | RBRACKET | RBRACE => depth -= 1,
                _ => {}
            }
            if depth != 0 || self.glued(k) {
                continue;
            }
            let w = self.word(k);
            let next = self.word(k + 1);
            let prev = self.word(k - 1);
            let clause = if bdef {
                matches!(
                    w.as_str(),
                    "implementation"
                        | "persistent"
                        | "draft"
                        | "lock"
                        | "total"
                        | "etag"
                        | "authorization"
                        | "early"
                        | "late"
                        | "with"
                        | "query"
                        | "extensible"
                ) && !(w == "etag" && prev == "total")
                    && !(w == "draft" && prev == "with")
            } else {
                (w == "as" && matches!(next.as_str(), "select" | "projection"))
                    || w == "provider"
                    || w == "association"
                    || w == "composition"
                    || (w == "with" && next == "parameters")
                    || (matches!(w.as_str(), "inner" | "left" | "right" | "cross")
                        && !matches!(
                            prev.as_str(),
                            "inner" | "left" | "right" | "cross" | "outer"
                        ))
                    || (w == "join"
                        && !matches!(
                            prev.as_str(),
                            "inner" | "left" | "right" | "cross" | "outer"
                        ))
            };
            if clause {
                in_params = w == "with" && next == "parameters";
                starts.push(k);
            } else if in_params && self.kind(k) == COMMA {
                params.push(k + 1);
            }
        }
        let pads = if self.aligned && !bdef {
            self.association_pads(&starts, last)
        } else {
            Default::default()
        };
        let mut docs = vec![self.t(first)];
        let mut k = first + 1;
        let mut segment: Vec<Doc> = Vec::new();
        let mut segments: Vec<Doc> = Vec::new();
        let flush = |segment: &mut Vec<Doc>, segments: &mut Vec<Doc>, docs: &mut Vec<Doc>| {
            if segments.is_empty() && docs.len() == 1 {
                docs.append(segment);
            } else {
                // The clause's own line break stays outside the group, so the
                // group breaks only when the clause does not fit.
                let mut seg = std::mem::take(segment);
                let rest = seg.split_off(1);
                let head = seg.pop().unwrap_or_else(b::empty);
                segments.push(b::indent(vec![head, b::group(rest)]));
            }
        };
        // CDS-ALIGN-004: the name list of a `define view`, one name per line.
        let names = if self.aligned && !bdef {
            self.name_list(first, last)
        } else {
            None
        };
        // Inside a block written on the header's line (`{ … }`), nothing breaks.
        let mut braces = 0i32;
        while k <= last {
            if let Some((lp, rp, items)) = &names
                && k == *lp
            {
                let mut inner = Vec::new();
                for &(a, e) in items {
                    inner.push(b::hard(self.stream, TokenIndex(a as u32)));
                    for j in a..e {
                        if j > a {
                            inner.push(self.sp(j));
                        }
                        inner.push(self.t(j));
                    }
                }
                segment.push(b::hard(self.stream, TokenIndex(*lp as u32)));
                segment.push(self.t(*lp));
                segment.push(b::indent(inner));
                segment.push(b::hard(self.stream, TokenIndex(*rp as u32)));
                segment.push(self.t(*rp));
                k = rp + 1;
                continue;
            }
            match self.kind(k) {
                LBRACE => braces += 1,
                RBRACE => braces -= 1,
                _ => {}
            }
            if starts.contains(&k) {
                flush(&mut segment, &mut segments, &mut docs);
                segment.push(self.hl(k));
                segment.push(self.t(k));
            } else if params.contains(&k) {
                segment.push(b::indent(vec![self.hl(k), self.t(k)]));
            } else if !bdef
                && braces == 0
                && matches!(self.word(k).as_str(), "and" | "or" | "on")
                && !self.glued(k)
            {
                if let Some(&p) = pads.get(&k) {
                    segment.push(b::spaces(p));
                }
                // Long conditions break before on/and/or, two levels deeper.
                segment.push(b::indent(vec![b::indent(vec![self.ln(k), self.t(k)])]));
            } else {
                match pads.get(&k) {
                    Some(&p) => segment.push(b::spaces(p + 1)),
                    None => segment.push(self.sp(k)),
                }
                segment.push(self.t(k));
            }
            k += 1;
        }
        flush(&mut segment, &mut segments, &mut docs);
        docs.extend(segments);
        Some(b::concat(docs))
    }

    /// CDS-ALIGN-004: `define view Name ( a, b )`: the brackets and the
    /// names with their commas (`(start, end)` per name). `None` unless every
    /// bracket and name can start a line (none of them glued) and no comment
    /// stands inside.
    fn name_list(&self, first: usize, last: usize) -> Option<NameList> {
        if self.word(first + 1) != "view" || self.word(first + 2) == "entity" {
            return None;
        }
        let lp = first + 3;
        if lp > last || self.kind(lp) != LPAREN || self.glued(lp) {
            return None;
        }
        let toks = &self.stream.tokens;
        let mut items = Vec::new();
        let mut a = lp + 1;
        let mut rp = None;
        for k in lp + 1..=last {
            match self.kind(k) {
                LPAREN | LBRACKET => return None,
                COMMA => {
                    items.push((a, k + 1));
                    a = k + 1;
                }
                RPAREN => {
                    items.push((a, k));
                    rp = Some(k);
                    break;
                }
                _ => {}
            }
        }
        let rp = rp?;
        if self.glued(rp) || items.iter().any(|&(a, e)| a >= e || self.glued(a)) {
            return None;
        }
        let (from, to) = (toks[lp].start, toks[rp].end);
        if self
            .stream
            .trivia
            .iter()
            .any(|t| t.is_comment() && t.start > from && t.end < to)
        {
            return None;
        }
        Some((lp, rp, items))
    }

    /// CDS-ALIGN-001: padding in front of `as` and `on` of consecutive
    /// one-line associations and compositions, so both line up.
    fn association_pads(
        &self,
        starts: &[usize],
        last: usize,
    ) -> std::collections::HashMap<usize, usize> {
        let mut pads = std::collections::HashMap::new();
        // (index of `as`, index of `on`, width up to `as`, width from `as` up
        // to `on`, width from `on`) of each alignable clause, in runs of
        // consecutive clauses.
        let mut runs: Vec<Vec<AssocRow>> = vec![Vec::new()];
        for (n, &s) in starts.iter().enumerate() {
            let end = starts.get(n + 1).map_or(last, |&e| e - 1);
            let w = self.word(s);
            let row = if matches!(w.as_str(), "association" | "composition") {
                self.association_row(s, end)
            } else {
                None
            };
            match row {
                Some(r) => runs.last_mut().unwrap().push(r),
                None => runs.push(Vec::new()),
            }
        }
        for run in runs.iter().filter(|r| r.len() > 1) {
            let col_as = run.iter().map(|r| r.2).max().unwrap_or(0);
            let col_on = run.iter().map(|r| r.3).max().unwrap_or(0);
            // Every padded clause must stay on one line; otherwise its `on`
            // would break and the next run would not count it.
            if col_as + col_on > 120
                || run
                    .iter()
                    .any(|r| self.indent + col_as + 1 + col_on + 1 + r.4 > self.width)
            {
                continue;
            }
            for &(a, o, wa, wo, _) in run {
                pads.insert(a, col_as - wa);
                pads.insert(o, col_on - wo);
            }
        }
        pads
    }

    /// `association … to target as _alias on …` without comments that fits
    /// on one line (the header prints it there, whatever the input's line
    /// breaks): the positions of `as` and `on` and the widths around them.
    fn association_row(&self, s: usize, end: usize) -> Option<(usize, usize, usize, usize, usize)> {
        let toks = &self.stream.tokens;
        if self
            .stream
            .trivia
            .iter()
            .any(|t| t.is_comment() && t.start >= toks[s].start && t.end <= toks[end].end)
        {
            return None;
        }
        let mut depth = 0i32;
        let (mut a, mut o) = (None, None);
        for k in s..=end {
            match self.kind(k) {
                LPAREN | LBRACKET => depth += 1,
                RPAREN | RBRACKET => depth -= 1,
                _ => {}
            }
            if depth == 0 && !self.glued(k) {
                match self.word(k).as_str() {
                    "as" if a.is_none() => a = Some(k),
                    "on" if a.is_some() && o.is_none() => o = Some(k),
                    _ => {}
                }
            }
        }
        let (a, o) = (a?, o?);
        let width = |from: usize, to: usize| -> usize {
            (from..to)
                .map(|k| {
                    let gap = usize::from(k > from && !self.glued(k));
                    gap + toks[k].text(self.text).chars().count()
                })
                .sum()
        };
        let line = width(s, end + 1);
        // A clause that does not fit breaks before `on` anyway.
        if self.indent + line > self.width {
            return None;
        }
        Some((a, o, width(s, a), width(a, o), width(o, end + 1)))
    }

    // ---- select-list elements ------------------------------------------

    /// A DDLS select-list element that contains `case` (CDS-ELEM-001). A case
    /// with two or more `when` always breaks: each `when` / `else` one level
    /// deeper than its `case`, `end` at the level of its `case`. A case with
    /// one `when` stays on one line if it fits. Elements without `case`
    /// return `None` and keep the author's line breaks.
    pub fn element(&self, first: usize, last: usize) -> Option<Doc> {
        if !(first..=last).any(|k| self.word(k) == "case") {
            return None;
        }
        // Count `when`s per case (by the index of its `case`).
        let mut stack: Vec<usize> = Vec::new();
        let mut whens = std::collections::HashMap::new();
        for k in first..=last {
            match self.word(k).as_str() {
                "case" => stack.push(k),
                "when" => *whens.entry(*stack.last()?).or_insert(0) += 1,
                "end" => {
                    stack.pop()?;
                }
                _ => {}
            }
        }
        if !stack.is_empty() {
            return None;
        }
        let nest = |d: usize, doc: Doc| (0..d).fold(doc, |x, _| b::indent(vec![x]));
        let mut docs = vec![self.t(first)];
        let mut spans = Vec::new();
        let mut cases: Vec<(usize, bool)> = Vec::new(); // (case index, multi-when)
        if self.word(first) == "case" {
            cases.push((first, whens.get(&first).copied().unwrap_or(0) > 1));
        }
        for k in first + 1..=last {
            let w = self.word(k);
            match w.as_str() {
                "when" | "else" if !cases.is_empty() && !self.glued(k) => {
                    let (_, multi) = *cases.last().unwrap();
                    let brk = if multi { self.hl(k) } else { self.ln(k) };
                    docs.push(nest(cases.len(), b::concat(vec![brk, self.t(k)])));
                }
                "end" if !cases.is_empty() && !self.glued(k) => {
                    let (case, multi) = cases.pop().unwrap();
                    spans.push(Self::span(case, k, "CDS-ELEM-001"));
                    let brk = if multi { self.hl(k) } else { self.ln(k) };
                    docs.push(nest(cases.len(), b::concat(vec![brk, self.t(k)])));
                }
                _ => {
                    docs.push(self.sp(k));
                    docs.push(self.t(k));
                    if w == "case" {
                        cases.push((k, whens.get(&k).copied().unwrap_or(0) > 1));
                    }
                }
            }
        }
        self.keep_spans(spans);
        Some(b::group(docs))
    }

    // ---- DCL -----------------------------------------------------------

    /// `grant select on <entity> [where …];` (DCL-GRANT-001): on one line if
    /// it fits; otherwise `where` starts a line one level deeper, and a long
    /// condition breaks before `and` / `or` two levels deeper.
    pub fn grant(&self, first: usize, last: usize) -> Option<Doc> {
        if self.word(first) != "grant" {
            return None;
        }
        let mut docs = vec![self.t(first)];
        let mut depth = 0i32;
        let mut in_where = false;
        for k in first + 1..=last {
            // Braces count too: a block written on one line stays in this
            // unit (`layout::units`), and a break inside it would make the
            // next run split it off (fuzz/found/cds-grant-brace-where.bin).
            match self.kind(k) {
                LPAREN | LBRACKET | LBRACE => depth += 1,
                RPAREN | RBRACKET | RBRACE => depth -= 1,
                _ => {}
            }
            let w = self.word(k);
            if depth == 0 && w == "where" && !self.glued(k) {
                in_where = true;
                docs.push(b::indent(vec![self.ln(k), self.t(k)]));
            } else if depth == 0 && in_where && matches!(w.as_str(), "and" | "or") && !self.glued(k)
            {
                docs.push(b::indent(vec![b::indent(vec![self.ln(k), self.t(k)])]));
            } else {
                docs.push(self.sp(k));
                docs.push(self.t(k));
            }
        }
        Some(b::group(docs))
    }
}

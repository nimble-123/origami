//! Grammar-based layout for CDS annotations (CDS-ANNO-001) and for the headers
//! of DDLS views and BDEF behavior definitions (CDS-HEAD-001). Anything these
//! parsers do not fully understand returns `None`, and the unit keeps the
//! structural layout (the author's line breaks, re-indented).

use crate::kinds::*;
use origami_layout::{Doc, builder as b};
use origami_syntax::{Language, TokenIndex, TokenStream};

pub struct G<'a> {
    pub text: &'a str,
    pub stream: &'a TokenStream,
    pub first: usize,
    pub blank: bool,
}

impl G<'_> {
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
                if k >= close {
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
        let mut docs = vec![self.t(first)];
        let mut k = first + 1;
        let mut segment: Vec<Doc> = Vec::new();
        let mut segments: Vec<Doc> = Vec::new();
        let flush = |segment: &mut Vec<Doc>, segments: &mut Vec<Doc>, docs: &mut Vec<Doc>| {
            if segments.is_empty() && docs.len() == 1 {
                docs.append(segment);
            } else {
                segments.push(b::indent(vec![b::group(std::mem::take(segment))]));
            }
        };
        while k <= last {
            if starts.contains(&k) {
                flush(&mut segment, &mut segments, &mut docs);
                segment.push(self.hl(k));
                segment.push(self.t(k));
            } else if params.contains(&k) {
                segment.push(b::indent(vec![self.hl(k), self.t(k)]));
            } else if !bdef && matches!(self.word(k).as_str(), "and" | "or") && !self.glued(k) {
                // Long conditions break before and/or, two levels deeper.
                segment.push(b::indent(vec![b::indent(vec![self.ln(k), self.t(k)])]));
            } else {
                segment.push(self.sp(k));
                segment.push(self.t(k));
            }
            k += 1;
        }
        flush(&mut segment, &mut segments, &mut docs);
        docs.extend(segments);
        Some(b::concat(docs))
    }
}

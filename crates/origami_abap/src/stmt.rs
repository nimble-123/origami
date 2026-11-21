//! Statement and expression parsing with direct layout.
//!
//! Each parser function consumes tokens and returns a [`Doc`]. A parser that
//! cannot handle its input returns `None`, and the statement falls back to the
//! generic layout (W003). Inside a recognized statement, a bracketed part that
//! the parser does not understand is printed as a raw sub-span: original line
//! breaks, re-indented with the statement.

use crate::kinds::*;
use origami_layout::{AlignItem, Doc, builder as b};
use origami_syntax::{TokenIndex, TokenStream};

pub struct Ctx<'a> {
    pub text: &'a str,
    pub stream: &'a TokenStream,
    /// Tokens with a leading own-line comment (they end alignment segments).
    pub commented: &'a [bool],
    /// Tokens recognized as keywords.
    pub keywords: Vec<usize>,
}

pub const PARAM_KEYWORDS: &[&str] = &[
    "EXPORTING",
    "IMPORTING",
    "CHANGING",
    "RECEIVING",
    "EXCEPTIONS",
];
const CONSTRUCTORS: &[&str] = &[
    "VALUE",
    "NEW",
    "CONV",
    "CORRESPONDING",
    "CAST",
    "REF",
    "EXACT",
    "COND",
    "SWITCH",
    "REDUCE",
    "FILTER",
];
const BINARY_OPS: &[&str] = &[
    "+", "-", "*", "/", "**", "&&", "DIV", "MOD", "BIT-AND", "BIT-OR", "BIT-XOR",
];
const ASSIGN_OPS: &[&str] = &["=", "?=", "+=", "-=", "*=", "/=", "&&="];

pub struct P<'c, 'a> {
    pub c: &'c mut Ctx<'a>,
    pub pos: usize,
    /// Index of the statement's terminating period (exclusive end).
    pub end: usize,
    /// First token of the statement, printed with blank-line handling.
    pub first: usize,
    pub blank: bool,
    /// The last operand ended with a call `( … )`.
    last_call: bool,
    /// Index of the terminating period; `end` stops before trailing pragmas.
    period_at: usize,
}

impl<'c, 'a> P<'c, 'a> {
    pub fn new(c: &'c mut Ctx<'a>, start: usize, end: usize, blank: bool) -> P<'c, 'a> {
        // Pragmas right before the period (`##NO_TEXT.`) belong to the
        // statement as a whole; parsers stop before them.
        let mut stop = end;
        while stop > start && c.stream.tokens[stop - 1].kind == PRAGMA {
            stop -= 1;
        }
        P {
            c,
            pos: start,
            end: stop,
            first: start,
            blank,
            last_call: false,
            period_at: end,
        }
    }

    // ---- token helpers -------------------------------------------------

    pub fn kind(&self, i: usize) -> u16 {
        if i < self.end {
            self.c.stream.tokens[i].kind
        } else {
            u16::MAX
        }
    }

    pub fn glued(&self, i: usize) -> bool {
        self.c.stream.tokens.get(i).is_some_and(|t| t.glued)
    }

    pub fn text(&self, i: usize) -> &'a str {
        self.c.stream.tokens[i].text(self.c.text)
    }

    pub fn is(&self, i: usize, word: &str) -> bool {
        i < self.end && self.kind(i) == WORD && self.text(i).eq_ignore_ascii_case(word)
    }

    pub fn upper(&self, i: usize) -> String {
        if i < self.end {
            self.text(i).to_ascii_uppercase()
        } else {
            String::new()
        }
    }

    pub fn t(&self, i: usize) -> Doc {
        if i == self.first && self.blank {
            b::token_keep_blank(TokenIndex(i as u32))
        } else {
            b::token(TokenIndex(i as u32))
        }
    }

    /// Keyword token: recorded for re-casing.
    pub fn kw(&mut self, i: usize) -> Doc {
        self.c.keywords.push(i);
        self.t(i)
    }

    /// One space before token `i` unless glued.
    pub fn sp(&self, i: usize) -> Doc {
        b::keep(self.c.stream, TokenIndex(i as u32))
    }

    /// Soft line before token `i` unless glued.
    pub fn ln(&self, i: usize) -> Doc {
        if self.glued(i) {
            b::empty()
        } else {
            b::soft(self.c.stream, TokenIndex(i as u32))
        }
    }

    /// Hard line before token `i` unless glued.
    pub fn hl(&self, i: usize) -> Doc {
        if self.glued(i) {
            b::empty()
        } else {
            b::hard(self.c.stream, TokenIndex(i as u32))
        }
    }

    pub fn at_end(&self) -> bool {
        self.pos >= self.end
    }

    /// Index of the bracket that closes the one at `open`.
    pub fn matching(&self, open: usize) -> Option<usize> {
        let mut depth = 0i32;
        for j in open..self.end {
            match self.kind(j) {
                LPAREN | LBRACKET | LBRACE => depth += 1,
                RPAREN | RBRACKET | RBRACE => {
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

    /// Tokens `from..=to` with their original gaps reduced to one space.
    pub fn flat(&self, from: usize, to: usize) -> Doc {
        let mut docs = vec![self.t(from)];
        for j in from + 1..=to {
            docs.push(self.sp(j));
            docs.push(self.t(j));
        }
        b::concat(docs)
    }

    /// Tokens `from..=to` as a raw sub-span: original line breaks, shifted.
    pub fn raw(&self, from: usize, to: usize) -> Doc {
        b::unknown(TokenIndex(from as u32), TokenIndex(to as u32), false)
    }

    // ---- expressions ---------------------------------------------------

    pub fn expr(&mut self) -> Option<Doc> {
        let first = self.operand()?;
        let mut parts = vec![first];
        while !self.at_end()
            && !self.glued(self.pos)
            && self.kind(self.pos) == WORD
            && BINARY_OPS.contains(&self.upper(self.pos).as_str())
        {
            let op = self.pos;
            let op_doc = if self.text(op).chars().all(|c| !c.is_alphabetic()) {
                self.t(op)
            } else {
                self.kw(op)
            };
            self.pos += 1;
            let rhs = self.operand()?;
            parts.push(b::concat(vec![self.ln(op), op_doc, self.sp(op + 1), rhs]));
        }
        Some(if parts.len() == 1 {
            parts.pop().unwrap()
        } else {
            b::group(parts)
        })
    }

    pub fn operand(&mut self) -> Option<Doc> {
        if self.at_end() {
            return None;
        }
        self.last_call = false;
        let i = self.pos;
        let mut docs = Vec::new();
        match self.kind(i) {
            CHAR_LITERAL | STRING_LITERAL | TEMPLATE_FULL => {
                docs.push(self.t(i));
                self.pos += 1;
            }
            TEMPLATE_HEAD => {
                // Templates stay as written (atomic, ABAP-WIDTH-001).
                let mut depth = 0;
                let mut j = i;
                loop {
                    match self.kind(j) {
                        TEMPLATE_HEAD => depth += 1,
                        TEMPLATE_TAIL => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        u16::MAX => return None,
                        _ => {}
                    }
                    j += 1;
                }
                docs.push(self.flat(i, j));
                self.pos = j + 1;
            }
            AT => {
                docs.push(self.t(i));
                self.pos += 1;
                if !self.glued(self.pos) {
                    return None;
                }
                if self.kind(self.pos) == LPAREN {
                    let close = self.matching(self.pos)?;
                    docs.push(self.flat(self.pos, close));
                    self.pos = close + 1;
                } else {
                    let inner = self.operand()?;
                    docs.push(inner);
                }
                return Some(b::concat(docs));
            }
            LPAREN => {
                let close = self.matching(i)?;
                self.pos = i + 1;
                match self.expr() {
                    Some(inner) if self.pos == close => docs.push(b::group(vec![
                        self.t(i),
                        self.sp(i + 1),
                        inner,
                        self.sp(close),
                        self.t(close),
                    ])),
                    _ => docs.push(self.raw(i, close)),
                }
                self.pos = close + 1;
            }
            WORD => {
                let w = self.upper(i);
                let ctor = CONSTRUCTORS.contains(&w.as_str())
                    && self.kind(i + 1) == WORD
                    && !self.glued(i + 1)
                    && self.kind(i + 2) == LPAREN
                    && self.glued(i + 2);
                if ctor {
                    docs.push(self.kw(i));
                    docs.push(self.sp(i + 1));
                    docs.push(self.t(i + 1));
                    self.pos = i + 2;
                    let body = self.parens(true)?;
                    docs.push(body);
                    self.last_call = true;
                } else if (w == "-" || w == "+") && !self.glued(i + 1) {
                    docs.push(self.t(i));
                    docs.push(self.sp(i + 1));
                    self.pos += 1;
                    let inner = self.operand()?;
                    docs.push(inner);
                    return Some(b::concat(docs));
                } else {
                    if BINARY_OPS.contains(&w.as_str()) || ASSIGN_OPS.contains(&w.as_str()) {
                        return None;
                    }
                    docs.push(self.t(i));
                    self.pos += 1;
                    // Inline declaration DATA( … ) / FINAL( … ).
                    if (w == "DATA" || w == "FINAL")
                        && self.kind(self.pos) == LPAREN
                        && self.glued(self.pos)
                    {
                        self.c.keywords.push(i);
                    }
                }
            }
            _ => return None,
        }
        // Postfix: selectors, calls, table expressions, offsets.
        while !self.at_end() && self.glued(self.pos) {
            let j = self.pos;
            match self.kind(j) {
                SELECTOR => {
                    docs.push(self.t(j));
                    self.pos += 1;
                    if !self.glued(self.pos) || self.kind(self.pos) != WORD {
                        return None;
                    }
                    docs.push(self.t(self.pos));
                    self.pos += 1;
                    self.last_call = false;
                }
                LPAREN => {
                    let body = self.parens(false)?;
                    docs.push(body);
                    self.last_call = !self.glued(j + 1);
                }
                LBRACKET => {
                    let close = self.matching(j)?;
                    docs.push(self.flat(j, close));
                    self.pos = close + 1;
                    self.last_call = false;
                }
                WORD => {
                    docs.push(self.t(j));
                    self.pos += 1;
                    self.last_call = false;
                }
                _ => break,
            }
        }
        Some(b::concat(docs))
    }

    /// A bracketed part starting at `self.pos` (which must be `(`): call
    /// arguments, constructor body, or a glued group such as `DATA(x)` or an
    /// offset/length `+3(2)`.
    fn parens(&mut self, constructor: bool) -> Option<Doc> {
        let lp = self.pos;
        let rp = self.matching(lp)?;
        self.pos = rp + 1;
        // Glued content: inline declaration, offset/length, text symbol.
        if self.glued(lp + 1) {
            return Some(self.flat(lp, rp));
        }
        if rp == lp + 1 {
            return Some(self.flat(lp, rp));
        }
        let saved_kw = self.c.keywords.len();
        let mut inner = P {
            c: &mut *self.c,
            pos: lp + 1,
            end: rp,
            first: self.first,
            blank: false,
            last_call: false,
            period_at: rp,
        };
        let parsed = if constructor {
            inner.ctor_body(lp, rp)
        } else {
            inner.call_args(lp, rp)
        };
        match parsed {
            Some(d) => Some(d),
            None => {
                self.c.keywords.truncate(saved_kw);
                Some(self.raw(lp, rp))
            }
        }
    }

    /// `( … )` with hanging content and the fallback after `(`.
    fn hanging(&self, lp: usize, rp: usize, content: Doc, fallback_content: Doc) -> Doc {
        let first = lp + 1;
        let hang = b::group(vec![
            self.t(lp),
            self.sp(first),
            b::hang(vec![content]),
            self.sp(rp),
            self.t(rp),
        ]);
        let fallback = b::group(vec![
            self.t(lp),
            b::cont(vec![self.hl(first), fallback_content]),
            self.sp(rp),
            self.t(rp),
        ]);
        b::choice(hang, fallback)
    }

    fn is_named_row(&self, i: usize) -> bool {
        self.kind(i) == WORD && self.is(i + 1, "=") && !self.glued(i + 1)
    }

    /// `name = value` rows up to a parameter keyword or the end.
    fn rows(&mut self) -> Option<Vec<AlignItem>> {
        let mut items = Vec::new();
        while !self.at_end() && !PARAM_KEYWORDS.contains(&self.upper(self.pos).as_str()) {
            let name = self.pos;
            if !self.is_named_row(name) || self.glued(name) {
                return None;
            }
            if !items.is_empty() && self.c.commented[name] {
                items.push(AlignItem::Break);
            }
            let eq = name + 1;
            self.pos = eq + 1;
            let value = self.expr()?;
            items.push(b::row(
                vec![self.t(name)],
                vec![self.sp(eq), self.t(eq), self.sp(eq + 1), value],
            ));
        }
        if items.is_empty() { None } else { Some(items) }
    }

    /// Call arguments between `lp` and `rp` (exclusive). `self.end == rp`.
    fn call_args(&mut self, lp: usize, rp: usize) -> Option<Doc> {
        let first = self.pos;
        if PARAM_KEYWORDS.contains(&self.upper(first).as_str()) {
            let mut groups: Vec<(usize, Vec<AlignItem>)> = Vec::new();
            while !self.at_end() {
                let k = self.pos;
                if !PARAM_KEYWORDS.contains(&self.upper(k).as_str()) {
                    return None;
                }
                self.c.keywords.push(k);
                self.pos += 1;
                groups.push((k, self.rows()?));
            }
            let mut hang = Vec::new();
            let mut fall = Vec::new();
            for (n, (k, rows)) in groups.into_iter().enumerate() {
                let g = b::concat(vec![
                    self.t(k),
                    self.sp(k + 1),
                    b::hang(vec![b::align(rows)]),
                ]);
                if n > 0 {
                    hang.push(self.ln(k));
                    fall.push(self.hl(k));
                }
                hang.push(g.clone());
                fall.push(g);
            }
            return Some(self.hanging(lp, rp, b::concat(hang), b::concat(fall)));
        }
        if self.is_named_row(first) {
            let rows = self.rows()?;
            // Rows must use up the whole list (a keyword group after named
            // rows without a leading keyword is not handled).
            if !self.at_end() {
                return None;
            }
            let a = b::align(rows);
            return Some(self.hanging(lp, rp, a.clone(), a));
        }
        // Single positional argument.
        let e = self.expr()?;
        if !self.at_end() {
            return None;
        }
        Some(b::group(vec![
            self.t(lp),
            self.sp(first),
            e,
            self.sp(rp),
            self.t(rp),
        ]))
    }

    /// Constructor body: rows `( … )`, component assignments, or one value.
    fn ctor_body(&mut self, lp: usize, rp: usize) -> Option<Doc> {
        let first = self.pos;
        if self.kind(first) == LPAREN && !self.glued(first) {
            // Table rows.
            let mut items = Vec::new();
            while !self.at_end() {
                let open = self.pos;
                if self.kind(open) != LPAREN {
                    return None;
                }
                let close = self.matching(open)?;
                let row = {
                    let mut inner = P {
                        c: &mut *self.c,
                        pos: open + 1,
                        end: close,
                        first: self.first,
                        blank: false,
                        last_call: false,
                        period_at: close,
                    };
                    if inner.is_named_row(open + 1) {
                        inner.rows().map(b::align)
                    } else {
                        inner.expr().filter(|_| inner.at_end())
                    }
                };
                let row_doc = match row {
                    Some(r) => b::group(vec![
                        self.t(open),
                        self.sp(open + 1),
                        r,
                        self.sp(close),
                        self.t(close),
                    ]),
                    None => self.raw(open, close),
                };
                if !items.is_empty() {
                    items.push(self.ln(open));
                }
                items.push(row_doc);
                self.pos = close + 1;
            }
            let content = b::concat(items);
            return Some(self.hanging(lp, rp, content.clone(), content));
        }
        if self.is_named_row(first) {
            let rows = self.rows()?;
            if !self.at_end() {
                return None;
            }
            let a = b::align(rows);
            return Some(self.hanging(lp, rp, a.clone(), a));
        }
        self.call_args(lp, rp)
    }

    // ---- logical expressions --------------------------------------------

    /// A logical expression: predicates joined by `AND`, `OR`, `EQUIV`. A
    /// broken expression breaks before each operator (ABAP-COND-001).
    pub fn logical(&mut self) -> Option<Doc> {
        let first = self.predicate()?;
        let mut parts = vec![first];
        while !self.at_end()
            && !self.glued(self.pos)
            && (self.is(self.pos, "AND") || self.is(self.pos, "OR") || self.is(self.pos, "EQUIV"))
        {
            let op = self.pos;
            let op_doc = self.kw(op);
            self.pos += 1;
            let rhs = self.predicate()?;
            parts.push(b::concat(vec![self.ln(op), op_doc, self.sp(op + 1), rhs]));
        }
        Some(if parts.len() == 1 {
            parts.pop().unwrap()
        } else {
            b::group(parts)
        })
    }

    fn predicate(&mut self) -> Option<Doc> {
        let mut docs = Vec::new();
        if self.is(self.pos, "NOT") {
            docs.push(self.kw(self.pos));
            docs.push(self.sp(self.pos + 1));
            self.pos += 1;
        }
        if self.kind(self.pos) == LPAREN && !self.glued(self.pos) {
            let lp = self.pos;
            let rp = self.matching(lp)?;
            let inner = {
                let mut q = P {
                    c: &mut *self.c,
                    pos: lp + 1,
                    end: rp,
                    first: self.first,
                    blank: false,
                    last_call: false,
                    period_at: rp,
                };
                q.logical().filter(|_| q.at_end())
            };
            docs.push(match inner {
                Some(d) => b::group(vec![
                    self.t(lp),
                    self.sp(lp + 1),
                    d,
                    self.sp(rp),
                    self.t(rp),
                ]),
                None => self.raw(lp, rp),
            });
            self.pos = rp + 1;
            return Some(b::concat(docs));
        }
        docs.push(self.expr()?);
        if self.at_end() {
            return Some(b::concat(docs));
        }
        let i = self.pos;
        let w = self.upper(i);
        let symbol = matches!(
            w.as_str(),
            "=" | "<>" | "<" | ">" | "<=" | ">=" | "><" | "=<" | "=>"
        );
        let named = matches!(
            w.as_str(),
            "EQ" | "NE"
                | "LT"
                | "GT"
                | "LE"
                | "GE"
                | "CO"
                | "CN"
                | "CA"
                | "NA"
                | "CS"
                | "NS"
                | "CP"
                | "NP"
                | "BYTE-CO"
                | "BYTE-CN"
                | "BYTE-CA"
                | "BYTE-NA"
                | "BYTE-CS"
                | "BYTE-NS"
                | "O"
                | "Z"
                | "M"
        );
        if (symbol || named) && self.kind(i) == WORD && !self.glued(i) {
            docs.push(self.sp(i));
            docs.push(if named { self.kw(i) } else { self.t(i) });
            docs.push(self.sp(i + 1));
            self.pos += 1;
            docs.push(self.expr()?);
            return Some(b::concat(docs));
        }
        if w == "IS" {
            docs.push(self.sp(i));
            docs.push(self.kw(i));
            self.pos += 1;
            if self.is(self.pos, "NOT") {
                docs.push(self.sp(self.pos));
                docs.push(self.kw(self.pos));
                self.pos += 1;
            }
            let k = self.pos;
            match self.upper(k).as_str() {
                "INITIAL" | "BOUND" | "ASSIGNED" | "SUPPLIED" | "REQUESTED" => {
                    docs.push(self.sp(k));
                    docs.push(self.kw(k));
                    self.pos += 1;
                }
                "INSTANCE" if self.is(k + 1, "OF") => {
                    docs.extend([
                        self.sp(k),
                        self.kw(k),
                        self.sp(k + 1),
                        self.kw(k + 1),
                        self.sp(k + 2),
                    ]);
                    self.pos += 2;
                    docs.push(self.operand()?);
                }
                _ => return None,
            }
            return Some(b::concat(docs));
        }
        let mut k = i;
        let negated = w == "NOT";
        if negated {
            k += 1;
        }
        if self.is(k, "BETWEEN") || self.is(k, "IN") {
            for j in i..=k {
                docs.push(self.sp(j));
                docs.push(self.kw(j));
            }
            self.pos = k + 1;
            docs.push(self.sp(self.pos));
            docs.push(self.expr()?);
            if self.is(k, "BETWEEN") {
                if !self.is(self.pos, "AND") {
                    return None;
                }
                let a = self.pos;
                docs.extend([self.sp(a), self.kw(a), self.sp(a + 1)]);
                self.pos += 1;
                docs.push(self.expr()?);
            }
        }
        Some(b::concat(docs))
    }

    // ---- statements ----------------------------------------------------

    /// The terminating period.
    pub fn period(&self) -> Doc {
        let mut docs = Vec::new();
        for i in self.end..=self.period_at {
            docs.push(self.sp(i));
            docs.push(self.t(i));
        }
        b::concat(docs)
    }

    /// Assignment `target op expr.` or standalone call `operand( … ).`.
    pub fn assignment_or_call(&mut self) -> Option<Doc> {
        let start = self.pos;
        let saved_kw = self.c.keywords.len();
        if let Some(target) = self.operand() {
            let op = self.pos;
            if !self.at_end()
                && !self.glued(op)
                && self.kind(op) == WORD
                && ASSIGN_OPS.contains(&self.text(op))
            {
                self.pos += 1;
                let rhs = self.expr()?;
                if !self.at_end() {
                    return None;
                }
                return Some(b::group(vec![
                    target,
                    self.sp(op),
                    self.t(op),
                    self.sp(op + 1),
                    b::cont(vec![rhs]),
                    self.period(),
                ]));
            }
            if self.at_end() && self.last_call {
                return Some(b::concat(vec![target, self.period()]));
            }
        }
        self.pos = start;
        self.c.keywords.truncate(saved_kw);
        None
    }
}

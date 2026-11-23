//! Statement and expression parsing with direct layout.
//!
//! Each parser function consumes tokens and returns a [`Doc`]. A parser that
//! cannot handle its input returns `None`, and the statement falls back to the
//! generic layout (W003). Inside a recognized statement, a bracketed part that
//! the parser does not understand is printed as a raw sub-span: original line
//! breaks, re-indented with the statement.

use crate::kinds::*;
use origami_layout::{AlignItem, Doc, builder as b};
use origami_syntax::{RuleId, RuleSpan, TokenIndex, TokenStream};

pub struct Ctx<'a> {
    pub text: &'a str,
    pub stream: &'a TokenStream,
    /// Tokens with a leading own-line comment (they end alignment segments).
    pub commented: &'a [bool],
    /// Tokens recognized as keywords.
    pub keywords: Vec<usize>,
    /// Record rule spans (explanations requested).
    pub explain: bool,
    /// Constructs and their layout rules; only filled when `explain`.
    pub rule_spans: Vec<RuleSpan>,
}

/// State of a [`Ctx`] before a parse attempt, to roll back a failed one.
#[derive(Debug, Clone, Copy)]
pub struct Mark {
    keywords: usize,
    spans: usize,
}

impl Ctx<'_> {
    pub fn mark(&self) -> Mark {
        Mark {
            keywords: self.keywords.len(),
            spans: self.rule_spans.len(),
        }
    }

    /// Drops the keywords and spans recorded since `m`.
    pub fn reset(&mut self, m: Mark) {
        self.keywords.truncate(m.keywords);
        self.rule_spans.truncate(m.spans);
    }

    /// Drops only the spans since `m`: a sub-span printed as written after a
    /// partly successful parse keeps its keywords (their case), as before.
    pub fn reset_spans(&mut self, m: Mark) {
        self.rule_spans.truncate(m.spans);
    }

    /// Records that tokens `first..=last` are laid out by `rule`.
    pub fn span(&mut self, first: usize, last: usize, rule: RuleId) {
        if self.explain {
            self.rule_spans.push(RuleSpan {
                first: TokenIndex(first as u32),
                last: TokenIndex(last as u32),
                rule,
            });
        }
    }
}

/// What a statement is aligned with its neighbors by (ABAP-ALIGN-004 to
/// ABAP-ALIGN-007): consecutive statements with the same key at the same
/// level form an alignment group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlignKey {
    /// An assignment to a component of this object (upper case: ABAP names
    /// are case-insensitive, and ABAP-CASE-002 may change their case).
    Assign(String),
    /// `METHODS name [ABSTRACT] FOR TESTING …`
    ForTesting,
    /// `METHODS name [FINAL] REDEFINITION`
    Redefinition,
    /// `ALIASES name FOR intf~comp`
    Aliases,
}

/// A statement that can be aligned with its neighbors: the part before the
/// anchor, the gap before the anchor, and the rest from the anchor to the
/// period.
pub struct Row {
    pub key: AlignKey,
    pub head: Vec<Doc>,
    pub gap: Doc,
    pub rest: Vec<Doc>,
    /// Width of the anchor for right alignment (assignment operators), else 0.
    pub anchor: usize,
}

impl Row {
    /// The statement on its own, laid out as without alignment.
    pub fn alone(self) -> Doc {
        let mut docs = self.head;
        docs.push(self.gap);
        docs.extend(self.rest);
        b::group(docs)
    }

    /// The statement as a row of an alignment group.
    pub fn aligned(self) -> AlignItem {
        b::row_right(self.head, self.anchor, vec![self.gap, b::group(self.rest)])
    }
}

/// A parsed statement.
pub enum Stmt {
    Doc(Doc),
    Row(Row),
}

impl Stmt {
    pub fn into_doc(self) -> Doc {
        match self {
            Stmt::Doc(d) => d,
            Stmt::Row(r) => r.alone(),
        }
    }
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
    "+", "-", "*", "/", "**", "&&", "&", "DIV", "MOD", "BIT-AND", "BIT-OR", "BIT-XOR",
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

/// Format options of embedded expressions in string templates.
const TEMPLATE_OPTIONS: &[&str] = &[
    "WIDTH",
    "ALIGN",
    "PAD",
    "CASE",
    "SIGN",
    "EXPONENT",
    "DECIMALS",
    "ZERO",
    "XSD",
    "STYLE",
    "CURRENCY",
    "NUMBER",
    "ALPHA",
    "DATE",
    "TIME",
    "TIMESTAMP",
    "TIMEZONE",
    "COUNTRY",
];

/// Predefined values of those options (`ALIGN = RIGHT`, `DATE = ISO`).
const TEMPLATE_VALUES: &[&str] = &[
    "LEFT",
    "RIGHT",
    "CENTER",
    "RAW",
    "UPPER",
    "LOWER",
    "ISO",
    "USER",
    "ENVIRONMENT",
    "YES",
    "NO",
    "IN",
    "OUT",
    "LEFTPLUS",
    "LEFTSPACE",
    "RIGHTPLUS",
    "RIGHTSPACE",
    "SIMPLE",
    "SIGN_AS_POSTFIX",
    "SCALE_PRESERVING",
    "SCIENTIFIC",
    "SCIENTIFIC_WITH_LEADING_ZERO",
    "SCALE_PRESERVING_SCIENTIFIC",
    "ENGINEERING",
    "SPACE",
];

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

    /// Format options of the embedded expressions of a string template
    /// `from..=to` (`{ d DATE = ISO }`, `{ n WIDTH = 10 ALIGN = RIGHT }`).
    fn mark_template_options(&mut self, from: usize, to: usize) {
        self.mark_phrases(from + 1, to);
        // Constructor bodies inside embedded expressions (`{ COND #( … ) }`).
        self.mark_ctor_keywords(from + 1, to, false);
        let part = |k: u16| matches!(k, TEMPLATE_HEAD | TEMPLATE_MIDDLE | TEMPLATE_TAIL);
        let mut depth = 0i32;
        for k in from + 1..to {
            match self.kind(k) {
                LPAREN | LBRACKET => depth += 1,
                RPAREN | RBRACKET => depth -= 1,
                _ => {}
            }
            if self.kind(k) == WORD && !self.glued(k) {
                let t = self.text(k).as_bytes();
                // Text symbol `TEXT-001`.
                let text_symbol = t.len() == 8
                    && t[..5].eq_ignore_ascii_case(b"text-")
                    && t[5..].iter().all(|b| b.is_ascii_digit());
                // Constructor operator `CONV type( … )`.
                let ctor = CTOR_OPS.contains(&self.upper(k).as_str())
                    && self.kind(k + 1) == WORD
                    && !self.glued(k + 1)
                    && self.kind(k + 2) == LPAREN
                    && self.glued(k + 2);
                if text_symbol || ctor {
                    self.c.keywords.push(k);
                }
            }
            let starts_expr = part(self.kind(k - 1));
            // Options follow the expression, outside its brackets.
            if depth != 0
                || starts_expr
                || self.kind(k) != WORD
                || !TEMPLATE_OPTIONS.contains(&self.upper(k).as_str())
                || !self.is(k + 1, "=")
                || self.glued(k + 1)
            {
                continue;
            }
            self.c.keywords.push(k);
            let v = k + 2;
            let option_ends = part(self.kind(v + 1)) || self.is(v + 2, "=");
            if self.kind(v) == WORD
                && option_ends
                && TEMPLATE_VALUES.contains(&self.upper(v).as_str())
            {
                self.c.keywords.push(v);
            }
        }
    }

    /// Keywords in arguments kept as written: an exception with a message
    /// (`zcx_x( MESSAGE e005(zmsg) WITH a )`, `zcx_x( USING MESSAGE )`) and
    /// fixed phrases.
    fn mark_raw_args(&mut self, lp: usize, rp: usize) {
        let k = lp + 1;
        if self.is(k, "USING") && self.is(k + 1, "MESSAGE") {
            self.c.keywords.extend([k, k + 1]);
        } else if self.is(k, "MESSAGE") && self.message_spec(k + 1) {
            self.c.keywords.push(k);
            let mut depth = 0i32;
            for j in k + 1..rp {
                match self.kind(j) {
                    LPAREN | LBRACKET => depth += 1,
                    RPAREN | RBRACKET => depth -= 1,
                    WORD if depth == 0
                        && !self.glued(j)
                        && matches!(self.upper(j).as_str(), "ID" | "TYPE" | "NUMBER" | "WITH") =>
                    {
                        self.c.keywords.push(j);
                    }
                    _ => {}
                }
            }
        }
        self.mark_phrases(lp + 1, rp);
    }

    /// A message after `MESSAGE`: `ID …`, or a short form `e005(zmsg)`.
    fn message_spec(&self, k: usize) -> bool {
        if self.is(k, "ID") {
            return true;
        }
        let t = self.text(k).as_bytes();
        self.kind(k) == WORD
            && t.len() == 4
            && b"aeiswxAEISWX".contains(&t[0])
            && t[1..].iter().all(u8::is_ascii_digit)
            && self.glued(k + 1)
            && self.kind(k + 1) == LPAREN
    }

    /// Fixed keyword phrases that are keywords wherever they stand:
    /// `GROUP SIZE`, `GROUP INDEX`, `IS [NOT] INITIAL|BOUND|ASSIGNED|SUPPLIED`,
    /// `IS [NOT] INSTANCE OF`.
    pub(crate) fn mark_phrases(&mut self, from: usize, to: usize) {
        for j in from..to {
            if self.kind(j) != WORD || self.glued(j) {
                continue;
            }
            if self.is(j, "GROUP") && (self.is(j + 1, "SIZE") || self.is(j + 1, "INDEX")) {
                self.c.keywords.extend([j, j + 1]);
            }
            if self.is(j, "IS") {
                let mut n = j + 1;
                if self.is(n, "NOT") {
                    n += 1;
                }
                let tail = match self.upper(n).as_str() {
                    "INITIAL" | "BOUND" | "ASSIGNED" | "SUPPLIED" => 1,
                    "INSTANCE" if self.is(n + 1, "OF") => 2,
                    _ => 0,
                };
                if tail > 0 {
                    self.c.keywords.extend(j..n + tail);
                }
            }
        }
    }

    /// Keywords of a table expression `[ KEY name COMPONENTS … ]` or
    /// `[ KEY name INDEX … ]` between `open` and `close`.
    fn mark_table_expr(&mut self, open: usize, close: usize) {
        let mut k = open + 1;
        if self.is(k, "TABLE") && self.is(k + 1, "KEY") {
            self.c.keywords.push(k);
            k += 1;
        }
        if k + 2 < close && self.is(k, "KEY") && self.kind(k + 1) == WORD && !self.glued(k + 1) {
            self.c.keywords.push(k);
            if self.is(k + 2, "COMPONENTS") || self.is(k + 2, "INDEX") {
                self.c.keywords.push(k + 2);
            }
        }
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
        // Unary `BIT-NOT x`.
        if self.kind(i) == WORD
            && self.upper(i) == "BIT-NOT"
            && i + 1 < self.end
            && !self.glued(i + 1)
        {
            let kw = self.kw(i);
            self.pos += 1;
            let rhs = self.operand()?;
            return Some(b::concat(vec![kw, self.sp(i + 1), rhs]));
        }
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
                self.mark_template_options(i, j);
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
                let mark = self.c.mark();
                match self.expr() {
                    Some(inner) if self.pos == close => docs.push(b::group(vec![
                        self.t(i),
                        self.sp(i + 1),
                        inner,
                        self.sp(close),
                        self.t(close),
                    ])),
                    _ => {
                        self.c.reset_spans(mark);
                        self.mark_phrases(i + 1, close);
                        docs.push(self.raw(i, close))
                    }
                }
                self.pos = close + 1;
            }
            WORD => {
                let w = self.upper(i);
                // The type may be qualified: `VALUE lo_obj->ty_list( … )`.
                let mut type_end = i + 2;
                while self.kind(type_end) == SELECTOR
                    && self.glued(type_end)
                    && self.kind(type_end + 1) == WORD
                    && self.glued(type_end + 1)
                {
                    type_end += 2;
                }
                let ctor = CONSTRUCTORS.contains(&w.as_str())
                    && self.kind(i + 1) == WORD
                    && !self.glued(i + 1)
                    && self.kind(type_end) == LPAREN
                    && self.glued(type_end);
                if ctor {
                    docs.push(self.kw(i));
                    docs.push(self.sp(i + 1));
                    for k in i + 1..type_end {
                        docs.push(self.t(k));
                    }
                    self.pos = type_end;
                    let body = self.parens(true)?;
                    docs.push(body);
                    self.last_call = true;
                } else if (w == "-" || w == "+") && i + 1 < self.end {
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
                    // Text symbol `TEXT-001` (an ID with letters keeps its case).
                    let t = self.text(i).as_bytes();
                    if t.len() == 8
                        && t[..5].eq_ignore_ascii_case(b"text-")
                        && t[5..].iter().all(|b| b.is_ascii_digit())
                    {
                        self.c.keywords.push(i);
                    }
                    // Inline declaration DATA( … ) / FINAL( … ) /
                    // FIELD-SYMBOL( … ).
                    if (w == "DATA" || w == "FINAL" || w == "FIELD-SYMBOL")
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
                    self.mark_table_expr(j, close);
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
        let saved_kw = self.c.mark();
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
            inner.call_args(lp, rp, true)
        };
        match parsed {
            Some(d) => Some(d),
            None => {
                self.c.reset(saved_kw);
                self.mark_raw_args(lp, rp);
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
    /// `call`: a method call (not a constructor); only calls record the
    /// ABAP-CALL-001 / ABAP-CALL-002 spans.
    fn call_args(&mut self, lp: usize, rp: usize, call: bool) -> Option<Doc> {
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
                let rows_from = self.pos;
                groups.push((k, self.rows()?));
                // `EXCEPTIONS … OTHERS = n`
                if self.is(k, "EXCEPTIONS") {
                    for j in rows_from..self.pos {
                        if self.is(j, "OTHERS") && self.is(j + 1, "=") && !self.glued(j + 1) {
                            self.c.keywords.push(j);
                        }
                    }
                }
            }
            if call {
                // Keywords: ABAP-CALL-002 for the list; the parameters of
                // one keyword: ABAP-CALL-001.
                self.c.span(lp, rp, "ABAP-CALL-002");
                for (n, (k, _)) in groups.iter().enumerate() {
                    let last = groups.get(n + 1).map_or(rp - 1, |g| g.0 - 1);
                    self.c.span(*k, last, "ABAP-CALL-001");
                }
            }
            let mut hang = Vec::new();
            let mut fall = Vec::new();
            for (n, (k, rows)) in groups.into_iter().enumerate() {
                // Shared: both layouts hold it (a copy would double the doc
                // at every nesting level).
                let g = b::shared(b::concat(vec![
                    self.t(k),
                    self.sp(k + 1),
                    b::hang(vec![b::align(rows)]),
                ]));
                if n > 0 {
                    // ABAP-CALL-002: a call with two or more parameter
                    // keywords always puts each keyword on its own line.
                    hang.push(if call { self.hl(k) } else { self.ln(k) });
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
            if call {
                self.c.span(lp, rp, "ABAP-CALL-001");
            }
            let a = b::shared(b::align(rows));
            return Some(self.hanging(lp, rp, a.clone(), a));
        }
        // Single positional argument: an expression, or a logical
        // expression (`xsdbool( a IS INITIAL )`).
        let mark = self.c.mark();
        let e = match self.expr().filter(|_| self.at_end()) {
            Some(e) => e,
            None => {
                self.c.reset(mark);
                self.pos = first;
                let l = self.logical()?;
                if !self.at_end() {
                    return None;
                }
                l
            }
        };
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
                let mark = self.c.mark();
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
                        // A parameter keyword ends the rows early; the row
                        // must still be used up, or its rest would be lost.
                        inner.rows().filter(|_| inner.at_end()).map(b::align)
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
                    None => {
                        self.c.reset_spans(mark);
                        self.raw(open, close)
                    }
                };
                if !items.is_empty() {
                    items.push(self.ln(open));
                }
                items.push(row_doc);
                self.pos = close + 1;
            }
            let content = b::shared(b::concat(items));
            self.c.span(lp, rp, "ABAP-VALUE-001");
            return Some(self.hanging(lp, rp, content.clone(), content));
        }
        if self.is_named_row(first) {
            let rows = self.rows()?;
            if !self.at_end() {
                return None;
            }
            self.c.span(lp, rp, "ABAP-VALUE-001");
            let a = b::shared(b::align(rows));
            return Some(self.hanging(lp, rp, a.clone(), a));
        }
        self.call_args(lp, rp, false)
    }

    /// Marks the keywords of a constructor body (`from..to`, bracket depth 0)
    /// and, recursively, of nested constructor bodies. With `words` false,
    /// only nested constructors are searched. A word right after a keyword
    /// that takes an operand (`FOR line IN`, `BASE base`) is a name; so is a
    /// word followed by `=` or by a glued `(`, `[` or selector.
    pub fn mark_ctor_keywords(&mut self, from: usize, to: usize, words: bool) {
        let mut prev: Option<String> = None;
        let mut i = from;
        while i < to {
            let k = self.kind(i);
            if k == TEMPLATE_HEAD {
                let mut depth = 0;
                let mut j = i;
                while j < to {
                    match self.kind(j) {
                        TEMPLATE_HEAD => depth += 1,
                        TEMPLATE_TAIL => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                if j < to {
                    self.mark_template_options(i, j);
                }
                prev = None;
                i = j + 1;
                continue;
            }
            if matches!(k, LPAREN | LBRACKET) {
                let Some(close) = self.matching(i).filter(|&c| c < to) else {
                    return;
                };
                // The type may be qualified: `FILTER lcl=>tt_type( … )`.
                let mut ty = i.saturating_sub(1);
                while ty >= from + 2
                    && self.kind(ty - 1) == SELECTOR
                    && self.glued(ty - 1)
                    && self.glued(ty)
                {
                    ty -= 2;
                }
                let ctor = k == LPAREN
                    && ty > from
                    && self.glued(i)
                    && !self.glued(ty)
                    && self.kind(ty - 1) == WORD
                    && CTOR_OPS.contains(&self.upper(ty - 1).as_str());
                if ctor && words {
                    self.c.keywords.push(ty - 1);
                }
                if k == LBRACKET {
                    self.mark_table_expr(i, close);
                }
                if !ctor && words && k == LPAREN && self.is(i + 1, "LINES") && self.is(i + 2, "OF")
                {
                    self.c.keywords.push(i + 1);
                    self.c.keywords.push(i + 2);
                }
                // A plain bracket (not a call) inside a constructor body:
                // a sub-condition or term, same keywords.
                let call = i > from && self.glued(i) && self.kind(i - 1) == WORD;
                if call && k == LPAREN {
                    self.mark_raw_args(i, close);
                }
                self.mark_ctor_keywords(i + 1, close, ctor || (words && k == LPAREN && !call));
                prev = None;
                i = close + 1;
                continue;
            }
            if words && k == WORD && !self.glued(i) {
                let w = self.upper(i);
                let next_eq = i + 1 < to && self.kind(i + 1) == WORD && self.text(i + 1) == "=";
                let next_call = i + 1 < to
                    && self.glued(i + 1)
                    && matches!(self.kind(i + 1), LPAREN | LBRACKET | SELECTOR);
                // A word equal to the keyword before it is a name
                // (`IN GROUP group`).
                let repeated = prev.as_deref() == Some(w.as_str());
                let operand = repeated
                    || prev.as_deref().is_some_and(|p| {
                        CTOR_TAKES_OPERAND.contains(&p)
                            && !matches!(
                                (p, w.as_str()),
                                (_, "NOT" | "THROW")
                                    | ("IN", "GROUP" | "FOR")
                                    | ("MAPPING", "FROM" | "TO")
                                    | ("EXCEPT", "IN" | "WHERE" | "USING")
                                    | ("FOR", "GROUPS")
                            )
                    });
                // A word right after an operator is an operand (`name = key`).
                let after_op = i > from
                    && self.kind(i - 1) == WORD
                    && matches!(
                        self.text(i - 1),
                        "=" | "<>" | "<" | ">" | "<=" | ">=" | "+" | "-" | "*" | "/" | "&&" | "?="
                    );
                // `MAPPING FROM ENTITY`, `MAPPING TO ENTITY`, `USING CONTROL`,
                // `GROUP SIZE`, `IS INSTANCE OF`
                let paired = match w.as_str() {
                    "GROUP" if self.is(i + 1, "SIZE") || self.is(i + 1, "INDEX") => true,
                    "SIZE" | "INDEX" if prev.as_deref() == Some("GROUP") => true,
                    "INSTANCE" => {
                        matches!(prev.as_deref(), Some("IS" | "NOT")) && self.is(i + 1, "OF")
                    }
                    "ENTITY" => matches!(prev.as_deref(), Some("FROM" | "TO")),
                    "CONTROL" => prev.as_deref() == Some("USING"),
                    _ => false,
                };
                if (paired || CTOR_WORDS.contains(&w.as_str()))
                    && !next_eq
                    && !next_call
                    && !operand
                    && !after_op
                {
                    self.c.keywords.push(i);
                    prev = Some(w);
                    i += 1;
                    continue;
                }
            }
            prev = None;
            i += 1;
        }
    }

    // ---- logical expressions --------------------------------------------

    /// A logical expression: predicates joined by `AND`, `OR`, `EQUIV`. A
    /// broken expression breaks before each operator (ABAP-COND-001).
    pub fn logical(&mut self) -> Option<Doc> {
        let start = self.pos;
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
            self.c.span(start, self.pos - 1, "ABAP-COND-001");
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
            let mark = self.c.mark();
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
                None => {
                    self.c.reset_spans(mark);
                    self.raw(lp, rp)
                }
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
                "INITIAL" | "BOUND" | "ASSIGNED" | "SUPPLIED" | "REQUESTED" | "NULL" => {
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

    /// ABAP-ALIGN-004: the object whose component the target `start..op`
    /// is (`ls_struc` for `ls_struc-comp`, `lo_obj` for `lo_obj->mv_attr`,
    /// `lo_obj->ms_s` for `lo_obj->ms_s-comp`), as ABAP Cleaner finds it:
    /// the name up to its last `-`. Only a plain name qualifies, optionally
    /// with offset and length (`ls_s-date+6(2)`); table expressions and
    /// calls do not.
    fn assign_key(&self, start: usize, op: usize) -> Option<AlignKey> {
        if start != self.first || self.kind(start) != WORD {
            return None;
        }
        let mut name = String::new();
        let mut k = start;
        while k < op && matches!(self.kind(k), WORD | SELECTOR) {
            if k > start && !self.glued(k) {
                return None;
            }
            name.push_str(self.text(k));
            k += 1;
        }
        // Length `(len)`.
        if k < op
            && !(k + 3 == op
                && self.kind(k) == LPAREN
                && self.kind(k + 1) == WORD
                && self.kind(k + 2) == RPAREN
                && (k..op).all(|j| self.glued(j)))
        {
            return None;
        }
        // Offset `+off`.
        let name = name.split('+').next().unwrap_or("");
        let cut = name.rfind('-').filter(|&c| c > 0)?;
        Some(AlignKey::Assign(name[..cut].to_ascii_uppercase()))
    }

    /// Assignment `target op expr.` or standalone call `operand( … ).`.
    pub fn assignment_or_call(&mut self) -> Option<Stmt> {
        let start = self.pos;
        let saved_kw = self.c.mark();
        if let Some(target) = self.operand() {
            let op = self.pos;
            if !self.at_end()
                && !self.glued(op)
                && self.kind(op) == WORD
                && ASSIGN_OPS.contains(&self.text(op))
            {
                self.pos += 1;
                let mut rhs = vec![self.expr()?];
                // Chained assignment `a = b = c.` (each `=` assigns to its left).
                let mut ops = vec![op];
                while !self.at_end()
                    && !self.glued(self.pos)
                    && self.kind(self.pos) == WORD
                    && self.text(self.pos) == "="
                    && self.text(op) == "="
                {
                    ops.push(self.pos);
                    self.pos += 1;
                    rhs.push(self.expr()?);
                }
                if !self.at_end() {
                    return None;
                }
                let mut docs = Vec::new();
                for (o, r) in ops.into_iter().zip(rhs) {
                    docs.push(self.sp(o));
                    docs.push(self.t(o));
                    docs.push(self.sp(o + 1));
                    docs.push(b::cont(vec![r]));
                }
                docs.push(self.period());
                if let Some(key) = self.assign_key(start, op) {
                    let gap = docs.remove(0);
                    return Some(Stmt::Row(Row {
                        key,
                        head: vec![target],
                        gap,
                        rest: docs,
                        anchor: self.text(op).chars().count(),
                    }));
                }
                docs.insert(0, target);
                return Some(Stmt::Doc(b::group(docs)));
            }
            if self.at_end() && self.last_call {
                return Some(Stmt::Doc(b::concat(vec![target, self.period()])));
            }
        }
        self.pos = start;
        self.c.reset(saved_kw);
        None
    }
}

/// Constructor operators whose body may hold keywords.
pub(crate) const CTOR_OPS: &[&str] = &[
    "VALUE",
    "CORRESPONDING",
    "REDUCE",
    "COND",
    "SWITCH",
    "FILTER",
    "CONV",
    "NEW",
    "REF",
    "EXACT",
    "CAST",
];

/// Keywords inside constructor bodies.
const CTOR_WORDS: &[&str] = &[
    "BASE",
    "FOR",
    "IN",
    "FROM",
    "TO",
    "INTO",
    "INDEX",
    "WHERE",
    "UNTIL",
    "WHILE",
    "THEN",
    "NEXT",
    "INIT",
    "LET",
    "MAPPING",
    "EXCEPT",
    "DISCARDING",
    "DUPLICATES",
    "DEEP",
    "APPENDING",
    "DEFAULT",
    "OPTIONAL",
    "WHEN",
    "ELSE",
    "USING",
    "KEY",
    "GROUPS",
    "GROUP",
    "OF",
    "BY",
    "ASCENDING",
    "DESCENDING",
    "WITHOUT",
    "MEMBERS",
    "STEP",
    "THROW",
    "AND",
    "OR",
    "NOT",
    "EQUIV",
    "IS",
    "INITIAL",
    "BOUND",
    "ASSIGNED",
    "SUPPLIED",
];

/// Keywords after which the next word is an operand.
const CTOR_TAKES_OPERAND: &[&str] = &[
    "BASE", "FOR", "IN", "INTO", "UNTIL", "WHILE", "THEN", "NEXT", "INIT", "LET", "MAPPING",
    "EXCEPT", "DEFAULT", "WHEN", "ELSE", "KEY", "GROUPS", "OF", "BY", "STEP",
];

#[cfg(test)]
mod tests {
    use super::Ctx;
    use origami_syntax::TokenStream;

    #[test]
    fn reset_drops_spans_of_a_failed_attempt() {
        let stream = TokenStream::default();
        let mut c = Ctx {
            text: "",
            stream: &stream,
            commented: &[],
            keywords: vec![1],
            explain: true,
            rule_spans: Vec::new(),
        };
        c.span(0, 3, "ABAP-STMT-001");
        let m = c.mark();
        c.keywords.push(2);
        c.span(1, 2, "ABAP-CALL-001");
        c.reset_spans(m);
        assert_eq!((c.keywords.len(), c.rule_spans.len()), (2, 1));
        c.span(1, 2, "ABAP-CALL-001");
        c.reset(m);
        assert_eq!((c.keywords.len(), c.rule_spans.len()), (1, 1));
        c.explain = false;
        c.span(1, 2, "ABAP-CALL-001");
        assert_eq!(c.rule_spans.len(), 1);
    }
}

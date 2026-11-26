//! Statement and expression parsing with direct layout.
//!
//! Each parser function consumes tokens and returns a [`Doc`]. A parser that
//! cannot handle its input returns `None`, and the statement falls back to the
//! generic layout (W003). Inside a recognized statement, a bracketed part that
//! the parser does not understand is printed as a raw sub-span: original line
//! breaks, re-indented with the statement.

use crate::kinds::*;
use origami_layout::{AlignItem, Doc, builder as b};
use origami_syntax::{
    CommentSyntax, RuleId, RuleSpan, Style, TokenIndex, TokenStream, TriviaKind, TriviaPosition,
};
use std::collections::HashMap;

pub struct Ctx<'a> {
    pub text: &'a str,
    /// The style (STYLE.md, "Styles").
    pub style: Style,
    pub stream: &'a TokenStream,
    /// Tokens with a leading own-line comment (they end alignment segments).
    pub commented: &'a [bool],
    /// Tokens followed by an end-of-line comment.
    pub eol_commented: &'a [bool],
    /// Tokens recognized as keywords.
    pub keywords: Vec<usize>,
    /// Record rule spans (explanations requested).
    pub explain: bool,
    /// Constructs and their layout rules; only filled when `explain`.
    pub rule_spans: Vec<RuleSpan>,
    /// The statement being parsed always spans several lines (a signature
    /// table, a broken chain); read by ABAP-BLANK-002.
    pub multi_line: bool,
    /// Aligned style, ABAP-DECL-002: spaces in front of the `END OF` that
    /// starts the statement being parsed, so `END` stands under `BEGIN`.
    pub end_gap: Option<usize>,
    /// Brackets already parsed by [`P::parens`], by opening bracket and
    /// kind: a parser that tries two readings of the brackets around them
    /// parses each inner bracket once, not once per attempt (which doubles
    /// at every level).
    pub parens: HashMap<(usize, bool), Parsed>,
    /// Logical expressions being parsed around the current one (brackets,
    /// `COND` in a condition); deeper than `MAX_COND_DEPTH`, a condition
    /// keeps the layout without ABAP-COND-001's alignment, so that the
    /// layout choices stay linear in the size of the statement.
    pub cond_depth: u8,
}

/// A bracket parsed by [`P::parens`]: its closing bracket, its doc, and the
/// keywords and spans it recorded.
pub struct Parsed {
    rp: usize,
    doc: Doc,
    keywords: Vec<usize>,
    spans: Vec<RuleSpan>,
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
    /// A one-line declaration with this keyword (`DATA`, `CONSTANTS`, …;
    /// ABAP-ALIGN-008).
    Decl(String),
}

/// A declaration in columns (ABAP-ALIGN-003, ABAP-ALIGN-008): the name, the
/// type, `VALUE …` and `READ-ONLY`, each optional part with the gap before
/// it. The last part ends with the statement's period or the item's comma.
#[derive(Clone)]
pub struct DeclCols {
    pub name: Vec<Doc>,
    pub ty: Vec<Doc>,
    pub value: Option<(Doc, Vec<Doc>)>,
    pub ro: Option<(Doc, Vec<Doc>)>,
}

impl DeclCols {
    /// Appends `end` (period or comma) to the last part.
    pub fn end_with(&mut self, end: Vec<Doc>) {
        if let Some((_, r)) = &mut self.ro {
            r.extend(end);
        } else if let Some((_, v)) = &mut self.value {
            v.extend(end);
        } else if !self.ty.is_empty() {
            self.ty.extend(end);
        } else {
            self.name.extend(end);
        }
    }

    /// The row, with a `VALUE` and a `READ-ONLY` column if `value_col` and
    /// `ro_col`; a part without its own column follows the part before it
    /// after one space.
    pub fn row(self, value_col: bool, ro_col: bool) -> AlignItem {
        let mut ty = self.ty;
        let mut head = Vec::new();
        let mut tail = Vec::new();
        if let Some((gap, v)) = self.value {
            if value_col {
                head.extend(v);
            } else {
                ty.push(gap);
                ty.extend(v);
            }
        }
        if let Some((gap, r)) = self.ro {
            if ro_col && !value_col {
                // The READ-ONLY column follows the type column directly.
                head.extend(r);
            } else if ro_col {
                tail.push(gap);
                tail.extend(r);
            } else if !head.is_empty() {
                head.push(gap);
                head.extend(r);
            } else {
                ty.push(gap);
                ty.extend(r);
            }
        }
        b::row_cols(vec![self.name, ty], head, tail)
    }
}

/// Which columns a group of declarations gets: `VALUE` and `READ-ONLY` each
/// when at least a fifth of the declarations have it (ABAP Cleaner's fill
/// ratio).
pub fn decl_columns(rows: &[&DeclCols]) -> (bool, bool) {
    let n = rows.len();
    let v = rows.iter().filter(|r| r.value.is_some()).count();
    let r = rows.iter().filter(|r| r.ro.is_some()).count();
    (v > 0 && v * 5 >= n, r > 0 && r * 5 >= n)
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
    /// A declaration's columns (key `Decl`), used when aligned.
    pub decl: Option<Box<DeclCols>>,
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

/// Where a logical expression stands, in the aligned style
/// (ABAP-COND-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CondAt {
    /// After a statement keyword (`IF`, `WHERE`, …): one condition per line
    /// from three conditions on; `AND` and `OR` start the lines, the first
    /// condition is indented to the column of the others.
    Keyword,
    /// After the `WHEN` at this token in `COND`: as `Keyword`, but `AND` and
    /// `OR` end in front of the condition column. The doc of the condition
    /// starts with the `WHEN`.
    When(usize),
    /// Inside brackets: on one line if it fits, else as `Keyword`; the
    /// width of what follows the closing bracket on its line.
    Nested(usize),
}

/// Logical expressions nested in one another (in brackets or in a `COND`)
/// that get the aligned layout of ABAP-COND-001 ([`Ctx::cond_depth`]).
const MAX_COND_DEPTH: u8 = 4;

/// A parsed logical expression ([`P::condition`]).
pub struct Condition {
    pub doc: Doc,
    /// Conditions at the outermost level.
    pub count: usize,
    /// One condition per line by rule (three or more): the aligned layouts
    /// alone and the layout without alignment, for a caller that decides
    /// the line break in front of the condition together with them.
    pub split: Option<(Doc, Doc)>,
}

/// A predicate of a logical expression: the leading `NOT`, the first
/// operand (or the whole predicate) and, for a comparison, the rest from
/// the space in front of the operator.
struct Pred {
    /// First token.
    from: usize,
    lead: Doc,
    term1: Doc,
    rest: Doc,
    /// A comparison whose first operand can be aligned.
    cmp: Option<Cmp>,
    not: bool,
    paren: bool,
    /// A comment inside: never padded.
    comment: bool,
}

/// Widths of a comparison's first operand and operator.
#[derive(Clone, Copy)]
struct Cmp {
    t1_w: usize,
    op_w: usize,
}

impl Pred {
    fn doc(&self) -> Doc {
        b::concat(vec![
            self.lead.clone(),
            self.term1.clone(),
            self.rest.clone(),
        ])
    }

    /// With padding `(NOT column, after the first operand, in front of the
    /// operator)` when broken.
    fn padded(&self, (not_pad, p1, p2): (usize, usize, usize)) -> Doc {
        b::concat(vec![
            b::break_pad(not_pad),
            self.lead.clone(),
            self.term1.clone(),
            b::break_pad(p1 + p2),
            self.rest.clone(),
        ])
    }
}

/// The operator structure of a logical expression: `EQUIV` binds weakest,
/// then `OR`, then `AND`. A group holds its operators (token indices, one
/// in front of each child after the first) and its children.
enum CondTree {
    Leaf(usize),
    Group(Vec<usize>, Vec<CondTree>),
}

fn cond_level(p: &P, op: usize) -> u8 {
    if p.is(op, "EQUIV") {
        0
    } else if p.is(op, "OR") {
        1
    } else {
        2
    }
}

/// The tree of predicates `a..=z`; `ops[k]` stands between predicates `k`
/// and `k + 1`.
fn cond_tree(p: &P, ops: &[usize], a: usize, z: usize) -> CondTree {
    let Some(level) = (a..z).map(|k| cond_level(p, ops[k])).min() else {
        return CondTree::Leaf(a);
    };
    let mut group_ops = Vec::new();
    let mut children = Vec::new();
    let mut from = a;
    for (k, &op) in ops.iter().enumerate().take(z).skip(a) {
        if cond_level(p, op) == level {
            children.push(cond_tree(p, ops, from, k));
            group_ops.push(op);
            from = k + 1;
        }
    }
    children.push(cond_tree(p, ops, from, z));
    CondTree::Group(group_ops, children)
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
        self.mark_ctor_keywords(from + 1, to, CtorScan::Nested);
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
                    let body = if w == "COND" || w == "SWITCH" {
                        self.cond_parens(w == "SWITCH")?
                    } else {
                        self.parens(true)?
                    };
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
                    let keys = if self.c.style == Style::Aligned {
                        self.table_keys(j, close)
                    } else {
                        None
                    };
                    docs.push(keys.unwrap_or_else(|| self.flat(j, close)));
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
        let key = (lp, constructor);
        if let Some(m) = self.c.parens.get(&key).filter(|m| m.rp == rp) {
            let doc = m.doc.clone();
            self.c.keywords.extend_from_slice(&m.keywords);
            self.c.rule_spans.extend_from_slice(&m.spans);
            return Some(doc);
        }
        let saved_kw = self.c.mark();
        let doc = self.parse_parens(lp, rp, constructor, saved_kw);
        let parsed = Parsed {
            rp,
            doc: doc.clone(),
            keywords: self.c.keywords[saved_kw.keywords..].to_vec(),
            spans: self.c.rule_spans[saved_kw.spans..].to_vec(),
        };
        self.c.parens.insert(key, parsed);
        Some(doc)
    }

    /// [`Self::parens`] for `lp..=rp`, not yet parsed.
    fn parse_parens(&mut self, lp: usize, rp: usize, constructor: bool, saved_kw: Mark) -> Doc {
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
            Some(d) => d,
            None => {
                self.c.reset(saved_kw);
                self.mark_raw_args(lp, rp);
                self.raw(lp, rp)
            }
        }
    }

    /// The body `( … )` of `COND` or `SWITCH` at `self.pos` (ABAP-COND-002);
    /// a body it does not understand falls back to [`Self::parens`].
    fn cond_parens(&mut self, switch: bool) -> Option<Doc> {
        let lp = self.pos;
        let rp = self.matching(lp)?;
        if self.glued(lp + 1) || rp == lp + 1 {
            return self.parens(true);
        }
        let saved = self.c.mark();
        let parsed = {
            let mut inner = P {
                c: &mut *self.c,
                pos: lp + 1,
                end: rp,
                first: self.first,
                blank: false,
                last_call: false,
                period_at: rp,
            };
            inner.cond_body(lp, rp, switch)
        };
        match parsed {
            Some(d) => {
                self.pos = rp + 1;
                Some(d)
            }
            None => {
                self.c.reset(saved);
                self.parens(true)
            }
        }
    }

    /// Whether a comment is attached inside `from..=to`: an end-of-line
    /// comment after `from..to`, or an own-line comment before
    /// `from + 1..=to`.
    pub(crate) fn has_comment(&self, from: usize, to: usize) -> bool {
        (from..to).any(|k| self.c.eol_commented[k]) || (from + 1..=to).any(|k| self.c.commented[k])
    }

    /// Width of `from..=to` printed by [`Self::flat`].
    pub(crate) fn flat_width(&self, from: usize, to: usize) -> usize {
        (from..=to)
            .map(|k| self.text(k).chars().count() + usize::from(k > from && !self.glued(k)))
            .sum()
    }

    /// `count` spaces before token `i` (which must not be glued).
    pub(crate) fn spaces(&self, i: usize, count: usize) -> Doc {
        b::concat((0..count).map(|_| self.sp(i)).collect())
    }

    /// `LET name = value … IN` at `self.pos`.
    fn let_expr(&mut self) -> Option<Doc> {
        let l = self.pos;
        let lead = self.kw(l);
        self.pos += 1;
        let mut defs = Vec::new();
        while !self.is(self.pos, "IN") {
            let name = self.pos;
            if !self.is_named_row(name) || self.glued(name) {
                return None;
            }
            self.pos = name + 2;
            let value = self.expr()?;
            if !defs.is_empty() {
                defs.push(self.ln(name));
            }
            defs.push(b::concat(vec![
                self.t(name),
                self.sp(name + 1),
                self.t(name + 1),
                self.sp(name + 2),
                value,
            ]));
        }
        let i = self.pos;
        let tail = self.kw(i);
        self.pos += 1;
        Some(b::group(vec![
            lead,
            self.sp(l + 1),
            b::hang(defs),
            self.sp(i),
            tail,
        ]))
    }

    /// A `THEN` or `ELSE` value: an expression or `THROW [RESUMABLE|SHORTDUMP]
    /// cx( … )`.
    fn cond_value(&mut self) -> Option<Doc> {
        let i = self.pos;
        if !self.is(i, "THROW") {
            return self.expr();
        }
        let mut docs = vec![self.kw(i)];
        self.pos += 1;
        if self.is(self.pos, "RESUMABLE") || self.is(self.pos, "SHORTDUMP") {
            docs.push(self.sp(self.pos));
            docs.push(self.kw(self.pos));
            self.pos += 1;
        }
        docs.push(self.sp(self.pos));
        docs.push(self.operand()?);
        Some(b::concat(docs))
    }

    /// `COND` / `SWITCH` body between `lp` and `rp` (ABAP-COND-002). One
    /// `WHEN`: on one line if it fits, else `WHEN`, `THEN` and `ELSE` below
    /// each other (aligned style, with an `ELSE`: always below each other). Several: one `WHEN … THEN …` per line with `THEN` and the
    /// values aligned (tabular), or, if that does not fit, `THEN` at the end
    /// of the `WHEN` line and each value on its own line, one level deeper.
    /// `LET … IN` and the `SWITCH` operand come first.
    fn cond_body(&mut self, lp: usize, rp: usize, switch: bool) -> Option<Doc> {
        struct Branch {
            when: usize,
            then: usize,
            /// `WHEN` and its condition.
            head: Doc,
            /// Conditions at the outermost level (ABAP-COND-001).
            count: usize,
            value: Doc,
        }
        // Parts before the first `WHEN`: (first token, doc).
        let mut lead: Vec<(usize, Doc)> = Vec::new();
        if self.is(self.pos, "LET") {
            let at = self.pos;
            lead.push((at, self.let_expr()?));
        }
        if switch {
            let at = self.pos;
            lead.push((at, self.expr()?));
        }
        let mut branches = Vec::new();
        while self.is(self.pos, "WHEN") {
            let when = self.pos;
            self.c.keywords.push(when);
            self.pos += 1;
            let (head, count) = if switch {
                let cond = self.expr()?;
                let head = b::concat(vec![self.t(when), self.sp(when + 1), b::cont(vec![cond])]);
                (head, 1)
            } else {
                let at = (self.c.style == Style::Aligned).then_some(CondAt::When(when));
                let Condition {
                    doc: head, count, ..
                } = self.condition(at)?;
                let head = if at.is_some() {
                    head
                } else {
                    b::concat(vec![self.t(when), self.sp(when + 1), b::cont(vec![head])])
                };
                (head, count)
            };
            let then = self.pos;
            if !self.is(then, "THEN") {
                return None;
            }
            self.c.keywords.push(then);
            self.pos += 1;
            let value = self.cond_value()?;
            branches.push(Branch {
                when,
                then,
                head: b::shared(head),
                count,
                value: b::shared(value),
            });
        }
        let mut otherwise = None;
        if self.is(self.pos, "ELSE") {
            let e = self.pos;
            self.c.keywords.push(e);
            self.pos += 1;
            otherwise = Some((e, b::shared(self.cond_value()?)));
        }
        if branches.is_empty() || !self.at_end() {
            return None;
        }
        self.c.span(lp, rp, "ABAP-COND-002");

        let content = if branches.len() == 1 {
            // One line if it fits; else every part below the previous one.
            // Aligned style: with an `ELSE`, always below each other.
            let below = self.c.style == Style::Aligned && otherwise.is_some();
            let brk = |p: &Self, i: usize| if below { p.hl(i) } else { p.ln(i) };
            let br = &branches[0];
            let mut docs = Vec::new();
            for (at, d) in &lead {
                if !docs.is_empty() {
                    docs.push(brk(self, *at));
                }
                docs.push(d.clone());
            }
            if !docs.is_empty() {
                docs.push(brk(self, br.when));
            }
            docs.extend([
                br.head.clone(),
                brk(self, br.then),
                self.t(br.then),
                self.sp(br.then + 1),
                b::cont(vec![br.value.clone()]),
            ]);
            if let Some((e, v)) = &otherwise {
                docs.extend([
                    brk(self, *e),
                    self.t(*e),
                    self.sp(e + 1),
                    b::cont(vec![v.clone()]),
                ]);
            }
            b::group(docs)
        } else {
            let mut head = Vec::new();
            for (at, d) in &lead {
                if !head.is_empty() {
                    head.push(self.hl(*at));
                }
                head.push(d.clone());
            }
            let first_when = branches[0].when;
            if !head.is_empty() {
                head.push(self.hl(first_when));
            }
            // Complex: `WHEN cond THEN`, the value one level deeper.
            let mut complex = Vec::new();
            for (n, br) in branches.iter().enumerate() {
                if n > 0 {
                    complex.push(self.hl(br.when));
                }
                complex.extend([
                    br.head.clone(),
                    self.sp(br.then),
                    self.t(br.then),
                    b::indent(vec![self.hl(br.then + 1), br.value.clone()]),
                ]);
            }
            if let Some((e, v)) = &otherwise {
                complex.extend([
                    self.hl(*e),
                    self.t(*e),
                    b::indent(vec![self.hl(e + 1), v.clone()]),
                ]);
            }
            let complex = b::concat(complex);
            // Tabular: `WHEN cond` printed flat, so its width is known; a
            // comment or a nested COND/SWITCH there rules it out, and so
            // does a condition on several lines (ABAP-COND-001).
            let tabular_ok = branches.iter().all(|br| {
                br.count < 3
                    && !self.has_comment(br.when, br.then + 1)
                    && !(br.when + 1..br.then).any(|k| self.is(k, "COND") || self.is(k, "SWITCH"))
            });
            let body = if tabular_ok {
                let widths: Vec<usize> = branches
                    .iter()
                    .map(|br| self.flat_width(br.when, br.then - 1))
                    .collect();
                let max = widths.iter().copied().max().unwrap_or(0);
                let mut tab = Vec::new();
                for (n, br) in branches.iter().enumerate() {
                    if n > 0 {
                        tab.push(self.hl(br.when));
                    }
                    tab.extend([
                        self.flat(br.when, br.then - 1),
                        self.spaces(br.then, max - widths[n] + 1),
                        self.t(br.then),
                        self.sp(br.then + 1),
                        b::cont(vec![br.value.clone()]),
                    ]);
                }
                if let Some((e, v)) = &otherwise {
                    // The ELSE value lines up with the THEN values.
                    tab.extend([
                        self.hl(*e),
                        self.t(*e),
                        self.spaces(e + 1, max + 2),
                        b::cont(vec![v.clone()]),
                    ]);
                }
                b::choice(b::concat(tab), complex)
            } else {
                complex
            };
            head.push(body);
            b::concat(head)
        };
        let content = b::shared(content);
        Some(self.hanging(lp, rp, content.clone(), content))
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

    /// Aligned `name = value` rows; in the aligned style two or more rows
    /// always go one per line (ABAP-CALL-001, ABAP-VALUE-001).
    fn rows_doc(&self, rows: Vec<AlignItem>) -> Doc {
        let n = rows
            .iter()
            .filter(|i| matches!(i, AlignItem::Row(_)))
            .count();
        if self.c.style == Style::Aligned && n > 1 {
            b::align_broken(rows)
        } else {
            b::align(rows)
        }
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
            if call && self.c.style == Style::Aligned {
                // Aligned style: one table for all groups, the keywords
                // padded to the longest, `=` aligned across the groups.
                let several = groups.len() > 1;
                let mut items = Vec::new();
                for (k, rows) in groups {
                    let mut first = true;
                    for it in rows {
                        items.push(match it {
                            AlignItem::Row(mut r) => {
                                r.lead = vec![if first {
                                    vec![self.t(k), b::space()]
                                } else {
                                    Vec::new()
                                }];
                                first = false;
                                AlignItem::Row(r)
                            }
                            other => other,
                        });
                    }
                }
                // ABAP-CALL-002: two or more keywords always break;
                // ABAP-CALL-001: so do two or more parameters.
                let params = items
                    .iter()
                    .filter(|i| matches!(i, AlignItem::Row(_)))
                    .count();
                let a = b::shared(if several || params > 1 {
                    b::align_broken(items)
                } else {
                    b::align(items)
                });
                return Some(self.hanging(lp, rp, a.clone(), a));
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
            let a = b::shared(self.rows_doc(rows));
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

    /// Aligned style (ABAP-VALUE-001): a table expression `[ … ]` from
    /// `open` to `close` with two or more key components, optionally after
    /// `KEY name [COMPONENTS]`, one component per line. `None` otherwise.
    fn table_keys(&mut self, open: usize, close: usize) -> Option<Doc> {
        if self.has_comment(open, close) || self.glued(open + 1) {
            return None;
        }
        let mark = self.c.mark();
        let mut k = open + 1;
        let mut head = Vec::new();
        let mut components = false;
        if self.is(k, "KEY") && self.kind(k + 1) == WORD && !self.glued(k + 1) {
            let mut e = k + 2;
            while e < close && self.glued(e) {
                e += 1;
            }
            for i in k..e {
                if i > k {
                    head.push(self.sp(i));
                }
                head.push(self.t(i));
            }
            k = e;
            if self.is(k, "COMPONENTS") {
                head.push(self.sp(k));
                head.push(self.t(k));
                k += 1;
                components = true;
            }
        }
        let parsed = {
            let mut inner = P {
                c: &mut *self.c,
                pos: k,
                end: close,
                first: self.first,
                blank: false,
                last_call: false,
                period_at: close,
            };
            if inner.is_named_row(k) {
                inner.rows().filter(|_| inner.at_end())
            } else {
                None
            }
        };
        let rows = match parsed {
            Some(r) if r.iter().filter(|i| matches!(i, AlignItem::Row(_))).count() > 1 => r,
            _ => {
                self.c.reset(mark);
                return None;
            }
        };
        self.c.span(open, close, "ABAP-VALUE-001");
        let rows = b::align_broken(rows);
        // `KEY name COMPONENTS` stays in front of the components; after
        // `KEY name` alone, the components start below `KEY`.
        let body = if head.is_empty() {
            rows
        } else if components {
            b::concat(vec![b::concat(head), self.sp(k), b::hang(vec![rows])])
        } else {
            b::concat(vec![b::concat(head), self.hl(k), rows])
        };
        Some(b::concat(vec![
            self.t(open),
            self.sp(open + 1),
            b::hang(vec![body]),
            self.sp(close),
            self.t(close),
        ]))
    }

    /// End (exclusive) of the iteration `FOR …` at `from`: the first `(` at
    /// bracket depth 0 that starts a row, not one that is an operand of the
    /// iteration (`WHERE ( … )`, `GROUP BY ( … )`, `THEN ( … )`).
    fn iteration_end(&self, from: usize) -> Option<usize> {
        const OPERAND_BEFORE: &[&str] = &[
            "WHERE", "BY", "THEN", "UNTIL", "WHILE", "=", "+", "-", "*", "/", "**", "&&", "AND",
            "OR", "NOT", "EQUIV", "DIV", "MOD", "<", ">", "<=", ">=", "<>", "EQ", "NE", "LT", "GT",
            "LE", "GE",
        ];
        let mut k = from + 1;
        while k < self.end {
            match self.kind(k) {
                LPAREN | LBRACKET => {
                    let prev = self.upper(k - 1);
                    if self.kind(k) == LPAREN
                        && !self.glued(k)
                        && !OPERAND_BEFORE.contains(&prev.as_str())
                    {
                        return Some(k);
                    }
                    k = self.matching(k)? + 1;
                }
                RPAREN | RBRACKET => return None,
                _ => k += 1,
            }
        }
        None
    }

    /// Aligned style (ABAP-VALUE-002): a `REDUCE` body `INIT … FOR … NEXT …`
    /// with each part on its own line, aligned under `INIT`.
    fn reduce_body(&mut self, lp: usize, rp: usize) -> Option<Doc> {
        let init = self.pos;
        // Word `w` at bracket depth 0 from `from` on.
        let find = |p: &Self, from: usize, w: &str| -> Option<usize> {
            let mut k = from;
            while k < p.end {
                match p.kind(k) {
                    LPAREN | LBRACKET => k = p.matching(k)? + 1,
                    _ if p.is(k, w) && !p.glued(k) => return Some(k),
                    _ => k += 1,
                }
            }
            None
        };
        let for_at = find(self, init + 1, "FOR")?;
        let next = find(self, for_at + 1, "NEXT")?;
        if next + 1 >= self.end || self.has_comment(init, self.end - 1) {
            return None;
        }
        let content = b::shared(b::concat(vec![
            self.flat(init, for_at - 1),
            self.hl(for_at),
            self.flat(for_at, next - 1),
            self.hl(next),
            self.flat(next, self.end - 1),
        ]));
        self.pos = self.end;
        self.c.span(lp, rp, "ABAP-VALUE-002");
        Some(self.hanging(lp, rp, content.clone(), content))
    }

    /// One table row `( … )` at `open`: its components flat if they fit.
    fn table_row(&mut self, open: usize) -> Option<(Doc, bool)> {
        let close = self.matching(open)?;
        let mark = self.c.mark();
        let mut named = false;
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
                named = true;
                // A parameter keyword ends the rows early; the row must
                // still be used up, or its rest would be lost.
                inner.rows().filter(|_| inner.at_end()).map(b::align)
            } else {
                inner.expr().filter(|_| inner.at_end())
            }
        };
        let doc = match row {
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
        self.pos = close + 1;
        Some((doc, named))
    }

    /// Aligned style (ABAP-VALUE-001): `BASE …` or `LET … IN` on the first
    /// line, then component assignments and table rows; broken when there
    /// are two or more components, two or more rows of which one has a
    /// component, or a `BASE`/`LET` with content after it. `None` for other
    /// bodies (a single value, `FOR`, …).
    fn ctor_body_aligned(&mut self, lp: usize, rp: usize) -> Option<Doc> {
        let mut parts: Vec<(usize, Doc)> = Vec::new();
        let mut lead = false;
        let start = self.pos;
        if self.is(start, "BASE") && !self.glued(start + 1) {
            let kw = self.kw(start);
            self.pos = start + 1;
            let e = self.expr()?;
            parts.push((start, b::concat(vec![kw, self.sp(start + 1), e])));
            lead = true;
        } else if self.is(start, "LET") {
            parts.push((start, self.let_expr()?));
            lead = true;
        }
        // ABAP-VALUE-002: `FOR …` on its own line, the rows below it.
        let mut iteration = false;
        if self.is(self.pos, "FOR") && !self.glued(self.pos + 1) {
            let from = self.pos;
            let to = self.iteration_end(from)?;
            if self.has_comment(from, to) {
                return None;
            }
            parts.push((from, self.flat(from, to - 1)));
            self.pos = to;
            iteration = true;
        }
        let mut comps = 0;
        if !iteration && !self.at_end() && self.is_named_row(self.pos) {
            let at = self.pos;
            let mut rows = Vec::new();
            while !self.at_end() && self.kind(self.pos) != LPAREN {
                let name = self.pos;
                if !self.is_named_row(name) || self.glued(name) {
                    return None;
                }
                self.pos = name + 2;
                let value = self.expr()?;
                rows.push(b::row(
                    vec![self.t(name)],
                    vec![
                        self.sp(name + 1),
                        self.t(name + 1),
                        self.sp(name + 2),
                        value,
                    ],
                ));
                comps += 1;
            }
            parts.push((at, b::align_broken(rows)));
        }
        let mut n_rows = 0;
        let mut named_row = false;
        while !self.at_end() {
            let open = self.pos;
            if self.kind(open) != LPAREN || self.glued(open) {
                return None;
            }
            let (d, named) = self.table_row(open)?;
            named_row |= named;
            n_rows += 1;
            parts.push((open, d));
        }
        let content = parts.len() - usize::from(lead);
        if iteration && n_rows == 0 {
            return None;
        }
        let broken = iteration
            || comps > 1
            || (n_rows > 1 && named_row)
            || (lead && content > 0)
            || (comps > 0 && n_rows > 0);
        if !broken || (lead && content == 0) {
            return None;
        }
        let mut docs = Vec::new();
        for (n, (at, d)) in parts.into_iter().enumerate() {
            if n > 0 {
                docs.push(self.hl(at));
            }
            docs.push(d);
        }
        let rule = if iteration {
            "ABAP-VALUE-002"
        } else {
            "ABAP-VALUE-001"
        };
        self.c.span(lp, rp, rule);
        let content = b::shared(b::concat(docs));
        Some(self.hanging(lp, rp, content.clone(), content))
    }

    /// Constructor body: rows `( … )`, component assignments, or one value.
    fn ctor_body(&mut self, lp: usize, rp: usize) -> Option<Doc> {
        let first = self.pos;
        if self.c.style == Style::Aligned {
            let mark = self.c.mark();
            let aligned = if self.is(first, "INIT") {
                self.reduce_body(lp, rp)
            } else {
                self.ctor_body_aligned(lp, rp)
            };
            if let Some(d) = aligned {
                return Some(d);
            }
            self.c.reset(mark);
            self.pos = first;
        }
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
            let a = b::shared(self.rows_doc(rows));
            return Some(self.hanging(lp, rp, a.clone(), a));
        }
        // `BASE lt ( 1 )`: an operand and a row, not the comparison `BASE LT …`.
        if self.is(first, "BASE") && first + 1 < rp && !self.glued(first + 1) {
            return None;
        }
        self.call_args(lp, rp, false)
    }

    /// Marks the keywords of the constructor bodies in `from..to` (see
    /// `CtorScan`). A word of `CTOR_WORDS` is a keyword only where the
    /// grammar allows it; in an operand position it is a name: right after a
    /// keyword that takes an operand (`FOR line IN`, `BASE base`) or after an
    /// operator, before `=` or a glued `(`, `[` or selector, as the whole
    /// content of a bracket or body, as the first operand of a body that does
    /// not start with a keyword (`SWITCH #( word WHEN …`), and after `EXCEPT`
    /// of `CORRESPONDING`.
    pub fn mark_ctor_keywords(&mut self, from: usize, to: usize, scan: CtorScan) {
        let words = matches!(scan, CtorScan::Body(_));
        let body_op = match scan {
            CtorScan::Body(op) => op,
            _ => None,
        };
        let mut prev: Option<String> = None;
        // After `EXCEPT` of `CORRESPONDING`: component names to the end.
        let mut names_only = false;
        // After `LET`: its `IN` ends the declarations; a keyword may follow.
        let mut in_let = false;
        let mut after_let_in = false;
        let mut i = from;
        while i < to {
            let k = self.kind(i);
            let let_in = std::mem::take(&mut after_let_in);
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
                if ctor && scan != CtorScan::Nested {
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
                let inner = if ctor {
                    let op = CTOR_OPS.iter().find(|o| **o == self.upper(ty - 1));
                    CtorScan::Body(op.copied())
                } else if k != LPAREN || call {
                    CtorScan::Nested
                } else if scan == CtorScan::Operand || prev.as_deref() == Some("BASE") {
                    // `BASE ( itab )`: an operand expression.
                    CtorScan::Operand
                } else if words {
                    CtorScan::Body(None)
                } else {
                    CtorScan::Nested
                };
                self.mark_ctor_keywords(i + 1, close, inner);
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
                // `IN GROUP group`; a word `group` alone is an iteration source.
                let group_ref = self.kind(i + 1) == WORD
                    && i + 1 < to
                    && !self.glued(i + 1)
                    && self.text(i + 1) != "="
                    && match self.upper(i + 1).as_str() {
                        "GROUP" => !self.is(i + 2, "BY"),
                        n => !CTOR_WORDS.contains(&n),
                    };
                let operand = !let_in
                    && (repeated
                        || prev.as_deref().is_some_and(|p| {
                            CTOR_TAKES_OPERAND.contains(&p)
                                && !matches!(
                                    (p, w.as_str()),
                                    (_, "NOT" | "THROW")
                                        | ("MAPPING", "FROM" | "TO")
                                        | ("EXCEPT", "IN" | "WHERE" | "USING")
                                        | ("FOR", "GROUPS")
                                )
                                && !(p == "IN" && w == "GROUP" && group_ref)
                        }));
                // The grammar allows no keyword here: the whole content of a
                // bracket or body, the first word of a body that does not
                // start with one, the components after `EXCEPT` of
                // `CORRESPONDING`, a word after `LET … IN` that starts no part.
                let name_position = names_only
                    || (w == "LET" && !(i + 2 < to && self.text(i + 2) == "="))
                    || (i == from && i + 1 == to)
                    || (i == from && body_op.is_some_and(|op| !leads(op, &w)))
                    || (let_in
                        && !(matches!(w.as_str(), "INIT" | "WHEN" | "FOR")
                            && body_op.is_none_or(|op| leads(op, &w))));
                // A word right after an operator is an operand (`name = key`).
                let after_op = i > from
                    && self.kind(i - 1) == WORD
                    && matches!(
                        self.text(i - 1),
                        "=" | "<>" | "<" | ">" | "<=" | ">=" | "+" | "-" | "*" | "/" | "&&" | "?="
                    );
                // `MAPPING FROM ENTITY`, `MAPPING TO ENTITY`, `USING CONTROL`,
                // `USING KEY`, `GROUP SIZE`, `IS INSTANCE OF`
                let paired = match w.as_str() {
                    "KEY" => prev.as_deref() == Some("USING"),
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
                    && !name_position
                {
                    self.c.keywords.push(i);
                    match w.as_str() {
                        "LET" => in_let = true,
                        "IN" if in_let => {
                            in_let = false;
                            after_let_in = true;
                        }
                        "EXCEPT" => names_only = body_op == Some("CORRESPONDING"),
                        _ => {}
                    }
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
        self.condition(None).map(|c| c.doc)
    }

    /// A logical expression and the number of conditions at its outermost
    /// level. With `at` (aligned style), in ABAP Cleaner's layout "Align
    /// logical expressions" (ABAP-COND-001): one condition per line from
    /// three conditions on (not in brackets), or when it does not fit;
    /// `AND`, `OR` and `EQUIV` in a column, the conditions in another, and
    /// the operators of consecutive comparisons aligned. If a line of that
    /// does not fit, the same without the operators aligned, and else the
    /// layout without `at`: on one line if it fits, otherwise broken before
    /// each operator at the continuation indent.
    pub fn condition(&mut self, at: Option<CondAt>) -> Option<Condition> {
        // After `WHEN`: the condition with the `WHEN` in front, so that the
        // aligned layout hangs at the `WHEN`.
        let when = match at {
            Some(CondAt::When(w)) => Some(w),
            _ => None,
        };
        let at = at.filter(|_| self.c.cond_depth < MAX_COND_DEPTH);
        self.c.cond_depth += 1;
        let parsed = self.condition_parts(at);
        self.c.cond_depth -= 1;
        let (preds, ops) = parsed?;
        let start = preds[0].from;
        let count = preds.len();
        let with_when = |p: &Self, d: Doc| match when {
            Some(w) => b::concat(vec![p.t(w), p.sp(w + 1), b::cont(vec![d])]),
            None => d,
        };
        let done = |doc, split| Condition { doc, count, split };
        if count == 1 {
            return Some(done(with_when(self, preds[0].doc()), None));
        }
        self.c.span(start, self.pos - 1, "ABAP-COND-001");
        let mut parts = vec![preds[0].doc()];
        for (n, &op) in ops.iter().enumerate() {
            parts.push(b::concat(vec![
                self.ln(op),
                self.t(op),
                self.sp(op + 1),
                preds[n + 1].doc(),
            ]));
        }
        let plain = b::group(parts);
        let Some(at) = at else {
            return Some(done(with_when(self, plain), None));
        };
        // From three conditions on, always one per line; else only when the
        // expression does not fit.
        let forced = !matches!(at, CondAt::Nested(_)) && count >= 3;
        let tree = cond_tree(self, &ops, 0, count - 1);
        // The last line must leave room for what follows it: ` THEN` after
        // `WHEN`, the period (or `)`) at the end.
        let reserve = if when.is_some() {
            " THEN".len()
        } else if self.at_end() {
            self.tail_width(at)
        } else {
            0
        };
        let hang = |p: &Self, cmp: bool| {
            let body = p.cond_node(&tree, &preds, at, forced, cmp, true);
            match when {
                Some(w) => b::hang(vec![p.t(w), p.sp(w + 1), body, b::reserve(reserve)]),
                None => b::hang(vec![body, b::reserve(reserve)]),
            }
        };
        let (with_ops, without) = (b::shared(hang(self, true)), b::shared(hang(self, false)));
        let aligned = b::choice(with_ops.clone(), without.clone());
        let plain = b::shared(with_when(self, plain));
        let doc = b::choice(with_ops, b::choice(without, plain.clone()));
        Some(if forced {
            done(doc, Some((aligned, plain)))
        } else {
            done(b::group(vec![doc]), None)
        })
    }

    /// The predicates of a logical expression and the operators between
    /// them.
    fn condition_parts(&mut self, at: Option<CondAt>) -> Option<(Vec<Pred>, Vec<usize>)> {
        let mut preds = vec![self.predicate_parts(at)?];
        let mut ops = Vec::new();
        while !self.at_end()
            && !self.glued(self.pos)
            && (self.is(self.pos, "AND") || self.is(self.pos, "OR") || self.is(self.pos, "EQUIV"))
        {
            let op = self.pos;
            self.c.keywords.push(op);
            self.pos += 1;
            ops.push(op);
            preds.push(self.predicate_parts(at)?);
        }
        Some((preds, ops))
    }

    /// One node of the condition tree (ABAP-COND-001), with the operators of
    /// comparisons aligned if `cmp`. `top`: the node is the whole
    /// expression, so `at` decides where its `AND`/`OR` stand; a nested node
    /// (an `AND` group inside `OR`) has them in a column in front of its
    /// conditions.
    fn cond_node(
        &self,
        node: &CondTree,
        preds: &[Pred],
        at: CondAt,
        forced: bool,
        cmp: bool,
        top: bool,
    ) -> Doc {
        let (ops, children) = match node {
            CondTree::Leaf(k) => return preds[*k].doc(),
            CondTree::Group(ops, children) => (ops, children),
        };
        let width = |op: usize| self.text(op).chars().count();
        let w = ops.iter().map(|&o| width(o)).max().unwrap_or(0);
        // After `WHEN`, the operators end in front of the condition column
        // (ABAP Cleaner); `EQUIV` is wider than `WHEN`.
        let when_gap = (top && matches!(at, CondAt::When(_)) && w <= 4).then(|| 4 - w);
        let pads = if cmp {
            self.cmp_pads(children, preds)
        } else {
            vec![None; children.len()]
        };
        let mut docs = Vec::new();
        if when_gap.is_none() {
            docs.push(b::break_pad(w + 1));
        }
        for (n, child) in children.iter().enumerate() {
            if n > 0 {
                let op = ops[n - 1];
                docs.push(if forced { self.hl(op) } else { self.ln(op) });
                if let Some(g) = when_gap {
                    docs.push(b::break_pad(g));
                }
                docs.push(self.t(op));
                docs.push(b::break_pad(w - width(op)));
                docs.push(self.sp(op + 1));
            }
            docs.push(match (child, pads[n]) {
                (CondTree::Leaf(k), Some(pad)) => preds[*k].padded(pad),
                (CondTree::Leaf(k), None) => preds[*k].doc(),
                (group, _) => b::hang(vec![self.cond_node(group, preds, at, forced, cmp, false)]),
            });
        }
        b::concat(docs)
    }

    /// Padding `(NOT column, after the first operand, in front of the
    /// operator)` of the children that are simple conditions, as ABAP
    /// Cleaner's "Align logical expressions" with its default options: in
    /// a run of consecutive simple conditions, a `NOT` column if one of
    /// them starts with `NOT`; in a run of consecutive comparisons, the
    /// first operands padded to the longest and the operators right-aligned,
    /// except in a row that would need more than 20 spaces in front of its
    /// operator.
    fn cmp_pads(
        &self,
        children: &[CondTree],
        preds: &[Pred],
    ) -> Vec<Option<(usize, usize, usize)>> {
        let simple = |c: &CondTree| match c {
            CondTree::Leaf(k) if !preds[*k].paren && !preds[*k].comment => Some(&preds[*k]),
            _ => None,
        };
        let mut out = vec![None; children.len()];
        let mut n = 0;
        while n < children.len() {
            let mut e = n;
            while e < children.len() && simple(&children[e]).is_some() {
                e += 1;
            }
            let run: Vec<&Pred> = children[n..e].iter().filter_map(simple).collect();
            let not_col = run.len() > 1 && run.iter().any(|p| p.not);
            for (k, p) in run.iter().enumerate() {
                out[n + k] = Some((if not_col && !p.not { 4 } else { 0 }, 0, 0));
            }
            let mut a = 0;
            while a < run.len() {
                let mut z = a;
                while z < run.len() && run[z].cmp.is_some() {
                    z += 1;
                }
                if z - a >= 2 {
                    let cmps: Vec<Cmp> = run[a..z].iter().filter_map(|p| p.cmp).collect();
                    let t1 = cmps.iter().map(|c| c.t1_w).max().unwrap_or(0);
                    let op = cmps.iter().map(|c| c.op_w).max().unwrap_or(0);
                    for (k, c) in (a..z).zip(&cmps) {
                        let (p1, p2) = (t1 - c.t1_w, op - c.op_w);
                        if p1 + p2 < 20
                            && let Some(pad) = &mut out[n + k]
                        {
                            pad.1 = p1;
                            pad.2 = p2;
                        }
                    }
                }
                a = z + 1;
            }
            n = e + 1;
        }
        out
    }

    /// Width of what follows a condition that ends at `self.end`: the
    /// period with its end-of-line comment, or ` )` and what follows that.
    fn tail_width(&self, at: CondAt) -> usize {
        let end: usize = (self.end..=self.period_at)
            .map(|k| self.text(k).chars().count() + usize::from(!self.glued(k)))
            .sum();
        match at {
            CondAt::Nested(after) => end + after,
            _ => end + self.eol_comment_width(self.period_at),
        }
    }

    /// A predicate in parts, for [`Self::condition`]: with `at`, a
    /// condition in brackets is laid out with [`CondAt::Nested`].
    fn predicate_parts(&mut self, at: Option<CondAt>) -> Option<Pred> {
        let from = self.pos;
        let mut lead = Vec::new();
        let not = self.is(self.pos, "NOT");
        if not {
            lead.push(self.kw(self.pos));
            lead.push(self.sp(self.pos + 1));
            self.pos += 1;
        }
        let done = |p: &Self, lead: Vec<Doc>, term1: Vec<Doc>, rest: Vec<Doc>, cmp, paren| Pred {
            from,
            lead: b::concat(lead),
            term1: b::shared(b::concat(term1)),
            rest: b::shared(b::concat(rest)),
            cmp,
            not,
            paren,
            comment: p.c.commented[from] || p.has_comment(from, p.pos - 1),
        };
        if self.kind(self.pos) == LPAREN && !self.glued(self.pos) {
            let lp = self.pos;
            let rp = self.matching(lp)?;
            // What follows `)` on its line, when the bracket ends the
            // condition.
            let nested = at.map(|at| {
                CondAt::Nested(if rp + 1 == self.end {
                    self.tail_width(at)
                } else if matches!(at, CondAt::When(_)) && self.is(rp + 1, "THEN") {
                    " THEN".len()
                } else {
                    0
                })
            });
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
                q.condition(nested).filter(|_| q.at_end())
            };
            let doc = match inner.map(|c| c.doc) {
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
            };
            self.pos = rp + 1;
            return Some(done(self, lead, vec![doc], Vec::new(), None, true));
        }
        let t1_from = self.pos;
        let mut term1 = vec![self.expr()?];
        if self.at_end() {
            return Some(done(self, lead, term1, Vec::new(), None, false));
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
        // The operator at `op` (also `IS`, `BETWEEN`, `IN`) after the first
        // operand `t1_from..=t1_to`, if that can be aligned.
        let cmp = |p: &Self, t1_to: usize, op: usize| {
            p.term_aligns(t1_from, t1_to).then(|| Cmp {
                t1_w: p.flat_width(t1_from, t1_to),
                op_w: p.text(op).chars().count(),
            })
        };
        if (symbol || named) && self.kind(i) == WORD && !self.glued(i) {
            let c = cmp(self, i - 1, i);
            let mut rest = vec![self.sp(i)];
            rest.push(if named { self.kw(i) } else { self.t(i) });
            rest.push(self.sp(i + 1));
            self.pos += 1;
            rest.push(self.expr()?);
            return Some(done(self, lead, term1, rest, c, false));
        }
        if w == "IS" {
            let c = cmp(self, i - 1, i);
            let mut rest = vec![self.sp(i), self.kw(i)];
            self.pos += 1;
            if self.is(self.pos, "NOT") {
                rest.push(self.sp(self.pos));
                rest.push(self.kw(self.pos));
                self.pos += 1;
            }
            let k = self.pos;
            match self.upper(k).as_str() {
                "INITIAL" | "BOUND" | "ASSIGNED" | "SUPPLIED" | "REQUESTED" | "NULL" => {
                    rest.push(self.sp(k));
                    rest.push(self.kw(k));
                    self.pos += 1;
                }
                "INSTANCE" if self.is(k + 1, "OF") => {
                    rest.extend([
                        self.sp(k),
                        self.kw(k),
                        self.sp(k + 1),
                        self.kw(k + 1),
                        self.sp(k + 2),
                    ]);
                    self.pos += 2;
                    rest.push(self.operand()?);
                }
                _ => return None,
            }
            return Some(done(self, lead, term1, rest, c, false));
        }
        let mut k = i;
        let negated = w == "NOT";
        if negated {
            k += 1;
        }
        if self.is(k, "BETWEEN") || self.is(k, "IN") {
            // `NOT BETWEEN`, `NOT IN`: the `NOT` ends the first operand's
            // column, as in ABAP Cleaner.
            if negated {
                term1.push(self.sp(i));
                term1.push(self.kw(i));
            }
            let c = cmp(self, k - 1, k);
            let mut rest = vec![self.sp(k), self.kw(k)];
            self.pos = k + 1;
            rest.push(self.sp(self.pos));
            rest.push(self.expr()?);
            if self.is(k, "BETWEEN") {
                if !self.is(self.pos, "AND") {
                    return None;
                }
                let a = self.pos;
                rest.extend([self.sp(a), self.kw(a), self.sp(a + 1)]);
                self.pos += 1;
                rest.push(self.expr()?);
            }
            return Some(done(self, lead, term1, rest, c, false));
        }
        Some(done(self, lead, term1, Vec::new(), None, false))
    }

    /// Width of the end-of-line comment after token `i` as the aligned style
    /// prints it: the blanks in front of it as written (ABAP-COMMENT-002),
    /// and a space added after `"` (ABAP-COMMENT-003).
    fn eol_comment_width(&self, i: usize) -> usize {
        if !self.c.eol_commented[i] {
            return 0;
        }
        // Trivia are in source order: those after token `i` up to the next.
        let trivia = &self.c.stream.trivia;
        let end = self.c.stream.tokens[i].end;
        let from = trivia.partition_point(|t| t.start < end);
        trivia[from..]
            .iter()
            .take_while(|t| t.anchor.0 as usize <= i + 1)
            .filter(|t| {
                t.is_comment() && t.position == TriviaPosition::Trailing && t.anchor.0 as usize == i
            })
            .map(|t| {
                let gap = &self.c.text[end as usize..t.start as usize];
                let gap = if gap.bytes().all(|b| b == b' ' || b == b'\t') {
                    gap.len().max(1)
                } else {
                    1
                };
                let text = t.text(self.c.text);
                let quote = matches!(
                    t.kind,
                    TriviaKind::Comment {
                        syntax: CommentSyntax::AbapQuote,
                        ..
                    }
                );
                let spaced = quote
                    .then(|| origami_syntax::spaced_comment(text))
                    .flatten();
                gap + spaced.map_or_else(|| text.chars().count(), |s| s.chars().count())
            })
            .max()
            .unwrap_or(0)
    }

    /// The first operand `from..=to` of a comparison can be aligned: it has
    /// no comment and is printed on one line whatever the width (no call
    /// with two or more parameters and no constructor expression, which
    /// break by rule).
    fn term_aligns(&self, from: usize, to: usize) -> bool {
        !self.has_comment(from, to)
            && (from..=to).filter(|&k| self.is(k, "=")).count() <= 1
            && !(from..=to).any(|k| {
                let w = self.upper(k);
                self.kind(k) == WORD
                    && (CONSTRUCTORS.contains(&w.as_str()) || PARAM_KEYWORDS.contains(&w.as_str()))
            })
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
                        decl: None,
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

/// What `mark_ctor_keywords` looks for in a range.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CtorScan {
    /// Only the constructors inside: the range itself is not a body.
    Nested,
    /// An operand expression (`BASE ( … )`): no keywords of its own, but the
    /// constructors inside.
    Operand,
    /// The body of a constructor operator; `None` for a row or bracket in a
    /// body, and where the operator is not known.
    Body(Option<&'static str>),
}

/// True when the body of `op` may start with the keyword `w`.
fn leads(op: &str, w: &str) -> bool {
    match op {
        "VALUE" | "NEW" => matches!(w, "BASE" | "LET" | "FOR"),
        "REDUCE" => matches!(w, "LET" | "INIT"),
        "COND" => matches!(w, "LET" | "WHEN"),
        "CORRESPONDING" => matches!(w, "BASE" | "DEEP" | "APPENDING"),
        "FILTER" => false,
        _ => w == "LET",
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
            style: origami_syntax::Style::Aligned,
            stream: &stream,
            commented: &[],
            eol_commented: &[],
            keywords: vec![1],
            explain: true,
            rule_spans: Vec::new(),
            multi_line: false,
            end_gap: None,
            parens: Default::default(),
            cond_depth: 0,
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

//! Printer: a Wadler/Prettier-style layout algorithm over the Doc IR.
//!
//! * A `Group` is printed flat when it contains no forced break and its flat
//!   content, plus the text that follows it up to the next possible line
//!   break, fits into the remaining width. Otherwise it is broken.
//! * Forced breaks: `HardLine`, a comment attached to a token that is not the
//!   group's first token, a trailing end-of-line comment, or raw text with a
//!   line break.
//! * `Hang` continues broken lines at the column where it starts.
//! * `Choice(a, b)` prints `a` when every line of `a` fits, else `b`.
//! * `AlignGroup` prints one row per line and pads each row's head to the
//!   widest head of its segment (anchors of different widths can be aligned
//!   at their right end); a segment falls back to single spaces when padding
//!   would overflow (see `AlignFit`) or an anchor is glued. Unaligned lines
//!   inside the group get their own line too.
//! * Comments are printed with their token. Own-line comments go on their own
//!   lines at the token's indent (`*` comments in column 1); end-of-line
//!   comments follow their token after one space and end the line.
//! * Indentation is written lazily, so lines never end in spaces.
//!
//! In identity mode the printer returns the source text unchanged (M0).
//!
//! Cost: a `Choice` is decided by printing its first branch into a trial
//! printer that starts from the current line only (the printer never looks
//! behind the last line break), and each decision is remembered for its
//! doc and printer state, so nested choices are decided once, not once per
//! enclosing trial (which would be exponential in their depth).

use crate::ir::{AlignFit, AlignItem, Doc, Inner};
use origami_syntax::{
    Case, CommentSyntax, Placement, SourceText, TokenIndex, TokenStream, TriviaKind, TriviaPosition,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Characters the lexers treat as blanks between tokens.
const BLANKS: [char; 3] = [' ', '\t', '\r'];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintOptions {
    pub width: u16,
    pub indent: u8,
    /// `None` leaves every token as written (CDS family).
    pub keyword_case: Option<Case>,
    /// Return the source text unchanged.
    pub identity: bool,
    /// Indent levels of a continuation line (`Doc::cont`): 1 in the aligned
    /// style, 2 in the compact style (ABAP-INDENT-001).
    pub cont_levels: u8,
    /// Aligned style (ABAP-COMMENT-002/-003, ABAP-BLANK-001): end-of-line
    /// comments keep the spaces in front of them, `"text` gets a space after
    /// the `"`.
    pub comment_style: bool,
    /// Tokens whose blank line after their comment lines is kept even though
    /// other blank lines before them are not (ABAP-BLANK-001, aligned).
    pub blank_after_comments: std::collections::BTreeSet<usize>,
    /// Blank lines added in front of these tokens and their comment lines
    /// (ABAP-BLANK-002); 2 may exceed the usual maximum of one.
    pub forced_blanks: std::collections::BTreeMap<usize, u8>,
    /// Replacement texts of comment lines, by trivia index (ABAP-DOC-001).
    /// The comment lines in front of one token use them only if no
    /// replacement ends past the width while being longer than the original.
    pub comment_overrides: BTreeMap<usize, String>,
    /// Hard limit of a line in characters (ABAP: 255, INVARIANTS 1.5). An
    /// own-line comment that would end past it, while its input line did
    /// not, keeps its text and is indented only as far as it fits
    /// (ABAP-COMMENT-003); see [`print_sparing`].
    pub line_limit: Option<usize>,
}

pub fn print(doc: &Doc, src: &SourceText, tokens: &TokenStream, opts: &PrintOptions) -> String {
    print_bounded(doc, src, tokens, opts, usize::MAX).expect("no output limit")
}

/// Like [`print`], but stops and returns `None` as soon as the output grows
/// beyond `max_len` bytes, so a layout that would blow up the input never
/// holds more than about `max_len` bytes in memory.
pub fn print_bounded(
    doc: &Doc,
    src: &SourceText,
    tokens: &TokenStream,
    opts: &PrintOptions,
    max_len: usize,
) -> Option<String> {
    print_sparing(doc, src, tokens, opts, max_len).map(|(out, _)| out)
}

/// Like [`print_bounded`], and also returns the trivia indices of the comment
/// lines kept within `PrintOptions::line_limit`, in order.
pub fn print_sparing(
    doc: &Doc,
    src: &SourceText,
    tokens: &TokenStream,
    opts: &PrintOptions,
    max_len: usize,
) -> Option<(String, Vec<usize>)> {
    if opts.identity {
        return Some((src.text().to_string(), Vec::new()));
    }
    let ctx = Ctx::new(src.text(), tokens, opts, max_len);
    let mut p = Printer::new(&ctx);
    p.run(vec![Cmd {
        indent: 0,
        mode: Mode::Break,
        doc,
    }]);
    if p.overflow {
        return None;
    }
    let out = p.finish();
    Some((out, ctx.spared.into_inner().into_iter().collect()))
}

/// A space, for joining aligned rows printed flat.
static SPACE_DOC: Doc = Doc(Inner::Space);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Mode {
    Flat,
    Break,
}

#[derive(Clone, Copy)]
struct Cmd<'d> {
    indent: usize,
    mode: Mode,
    doc: &'d Doc,
}

/// What a trial print of a `Choice` branch depends on: the doc, where it
/// starts, and the printer state. Of the output so far, the printer only
/// reads whether there is any, and of the current line its visual width
/// (`raw`) and its trailing blanks (removed before a line break).
#[derive(PartialEq, Eq, Hash)]
struct TrialKey {
    doc: *const Doc,
    indent: usize,
    mode: Mode,
    width: usize,
    col: usize,
    pending_indent: Option<usize>,
    pending_eol: bool,
    line_empty: bool,
    prev_blank: bool,
    out_empty: bool,
    line_width: usize,
    trailing_blanks: String,
}

/// A leading comment and whether a blank line preceded it.
struct Leading {
    trivia: usize,
    blank_before: bool,
}

struct Ctx<'a> {
    src: &'a str,
    tokens: &'a TokenStream,
    opts: &'a PrintOptions,
    leading: Vec<Vec<Leading>>,
    /// Blank line between the last leading comment (or the previous token)
    /// and the token itself.
    blank_before_token: Vec<bool>,
    trailing: Vec<Vec<usize>>,
    /// Output limit in bytes (see [`print_bounded`]).
    max_len: usize,
    /// Decided choices: whether the first branch fits.
    trials: RefCell<HashMap<TrialKey, bool>>,
    /// Comment lines kept within the line limit (`Printer::fit_line_limit`).
    spared: RefCell<BTreeSet<usize>>,
}

impl<'a> Ctx<'a> {
    fn new(
        src: &'a str,
        tokens: &'a TokenStream,
        opts: &'a PrintOptions,
        max_len: usize,
    ) -> Ctx<'a> {
        let n = tokens.tokens.len() + 1;
        let mut leading: Vec<Vec<Leading>> = (0..n).map(|_| Vec::new()).collect();
        let mut blank_before_token = vec![false; n];
        let mut trailing: Vec<Vec<usize>> = (0..n).map(|_| Vec::new()).collect();
        let mut newlines = 0usize;
        let mut last_anchor = usize::MAX;
        for (k, t) in tokens.trivia.iter().enumerate() {
            let a = t.anchor.0 as usize;
            if a != last_anchor {
                newlines = 0;
                last_anchor = a;
            }
            match (t.kind, t.position) {
                (TriviaKind::Newline, _) => newlines += 1,
                (TriviaKind::Whitespace, _) => {}
                (TriviaKind::Comment { .. }, TriviaPosition::Trailing) => {
                    trailing[a].push(k);
                    newlines = 0;
                }
                (TriviaKind::Comment { .. }, TriviaPosition::Leading) => {
                    leading[a].push(Leading {
                        trivia: k,
                        blank_before: newlines >= 2,
                    });
                    newlines = 0;
                }
            }
            if matches!(t.kind, TriviaKind::Newline) {
                blank_before_token[a] = newlines >= 2;
            } else if t.is_comment() {
                blank_before_token[a] = false;
            }
        }
        Ctx {
            src,
            tokens,
            opts,
            leading,
            blank_before_token,
            trailing,
            max_len,
            trials: RefCell::new(HashMap::new()),
            spared: RefCell::new(BTreeSet::new()),
        }
    }

    fn token_text(&self, i: TokenIndex) -> String {
        let t = &self.tokens.tokens[i.0 as usize];
        let text = t.text(self.src);
        match (t.keyword, self.opts.keyword_case) {
            (true, Some(Case::Upper)) => text.to_ascii_uppercase(),
            (true, Some(Case::Lower)) => text.to_ascii_lowercase(),
            // An identifier without lower-case letters is written in lower
            // case; one with lower-case letters (`lv_a`, `lvMixed`) is kept.
            _ if t.ident && !text.bytes().any(|b| b.is_ascii_lowercase()) => {
                text.to_ascii_lowercase()
            }
            _ => text.to_string(),
        }
    }

    fn has_leading_comments(&self, i: TokenIndex) -> bool {
        !self.leading[i.0 as usize].is_empty()
    }

    fn has_trailing_comments(&self, i: TokenIndex) -> bool {
        !self.trailing[i.0 as usize].is_empty()
    }

    fn raw_range(&self, first: TokenIndex, last: TokenIndex) -> (usize, usize) {
        let a = self.tokens.tokens[first.0 as usize].start as usize;
        let b = self.tokens.tokens[last.0 as usize].end as usize;
        (a, b)
    }

    /// Width of a raw unit printed on one line: unknown units shrink runs of
    /// spaces between tokens to one, verbatim and opaque units keep them.
    fn raw_width(&self, unknown: bool, first: TokenIndex, last: TokenIndex) -> usize {
        if !unknown {
            let (a, b) = self.raw_range(first, last);
            let text = &self.src[a..b];
            return text.split('\n').next().unwrap_or("").chars().count();
        }
        // Unknown units print single spaces between tokens; measure up to
        // the first line break, as printed.
        let mut w = 0;
        for k in first.0..=last.0 {
            let t = &self.tokens.tokens[k as usize];
            if k > first.0 && !glued_gap(self, k as usize) {
                let prev_end = self.tokens.tokens[k as usize - 1].end as usize;
                let gap = &self.src[prev_end..t.start as usize];
                if gap.contains('\n') {
                    break;
                }
                w += same_line_gap(gap).chars().count();
            }
            let text = self.token_text(TokenIndex(k));
            match text.find('\n') {
                Some(n) => return w + text[..n].chars().count(),
                None => w += text.chars().count(),
            }
        }
        w
    }

    /// Width of the docs up to their first possible line break (the shortest
    /// first line they can be printed with).
    fn min_first_line(&self, docs: &[Doc]) -> usize {
        let mut w = 0usize;
        let mut stack: Vec<&Doc> = docs.iter().rev().collect();
        while let Some(d) = stack.pop() {
            match &d.0 {
                Inner::Token { index, .. } => {
                    w += self.token_text(*index).chars().count();
                    if self.has_trailing_comments(*index) {
                        return w;
                    }
                }
                Inner::Space => w += 1,
                Inner::BreakPad(_) => {}
                Inner::SoftLine | Inner::SoftBreak | Inner::HardLine => return w,
                Inner::Group(ds)
                | Inner::Indent(ds)
                | Inner::IndentBy(_, ds)
                | Inner::LeadIndent(ds)
                | Inner::IndentCols(_, ds)
                | Inner::Cont(ds)
                | Inner::Hang(ds)
                | Inner::Concat(ds) => stack.extend(ds.iter().rev()),
                Inner::Choice(a, _) => stack.push(a),
                Inner::Shared(d) => stack.push(d),
                Inner::Fill(items) => {
                    for (sep, c) in items.iter().rev() {
                        stack.push(c);
                        stack.push(sep);
                    }
                }
                Inner::AlignGroup(items, ..) => {
                    if let Some(docs) = items.iter().find_map(AlignItem::docs) {
                        stack.extend(docs.rev());
                    }
                }
                Inner::Unknown { first, last, .. } => {
                    let (a, b) = self.raw_range(*first, *last);
                    w += self.raw_width(true, *first, *last);
                    if self.src[a..b].contains('\n') {
                        return w;
                    }
                }
                Inner::Verbatim { first, last, .. } | Inner::Opaque { first, last, .. } => {
                    let (a, b) = self.raw_range(*first, *last);
                    let line = self.src[a..b].split('\n').next().unwrap_or("");
                    w += line.chars().count();
                    if self.src[a..b].contains('\n') {
                        return w;
                    }
                }
            }
        }
        w
    }

    /// True when the doc can never be printed on one line.
    /// `end`: the doc's last token. An end-of-line comment after it breaks
    /// the line only after the doc, so it does not force the doc to break.
    fn forced(&self, doc: &Doc, first: &mut bool, end: Option<TokenIndex>) -> bool {
        let trailing = |i: TokenIndex| Some(i) != end && self.has_trailing_comments(i);
        match &doc.0 {
            Inner::HardLine => true,
            Inner::Token { index, .. } => {
                let lead = !*first && self.has_leading_comments(*index);
                *first = false;
                lead || trailing(*index)
            }
            Inner::Unknown { first: f, last, .. }
            | Inner::Verbatim { first: f, last, .. }
            | Inner::Opaque { first: f, last, .. } => {
                let lead = !*first && self.has_leading_comments(*f);
                *first = false;
                let (a, b) = self.raw_range(*f, *last);
                lead || self.src[a..b].contains('\n') || trailing(*last)
            }
            Inner::AlignGroup(items, _, broken) => {
                let rows = items
                    .iter()
                    .filter(|i| !matches!(i, AlignItem::Break))
                    .count();
                (*broken && rows > 1)
                    | items
                        .iter()
                        .filter_map(AlignItem::docs)
                        .any(|mut docs| docs.any(|d| self.forced(d, first, end)))
            }
            Inner::Group(ds)
            | Inner::Indent(ds)
            | Inner::IndentBy(_, ds)
            | Inner::LeadIndent(ds)
            | Inner::IndentCols(_, ds)
            | Inner::Cont(ds)
            | Inner::Hang(ds)
            | Inner::Concat(ds) => ds.iter().any(|d| self.forced(d, first, end)),
            Inner::Choice(a, _) => self.forced(a, first, end),
            Inner::Shared(d) => self.forced(d, first, end),
            Inner::Fill(items) => items.iter().any(|(_, c)| self.forced(c, first, end)),
            Inner::Space | Inner::SoftLine | Inner::SoftBreak | Inner::BreakPad(_) => false,
        }
    }

    fn is_forced(&self, doc: &Doc) -> bool {
        self.forced(doc, &mut true, last_token(doc))
    }

    /// Width of the docs printed flat, or `None` if they cannot be flat.
    fn flat_width(&self, docs: &[Doc]) -> Option<usize> {
        let end = docs.iter().rev().find_map(last_token);
        let mut first = true;
        if docs.iter().any(|d| self.forced(d, &mut first, end)) {
            return None;
        }
        let mut w = 0usize;
        let mut stack: Vec<&Doc> = docs.iter().rev().collect();
        while let Some(d) = stack.pop() {
            match &d.0 {
                Inner::Token { index, .. } => w += self.token_text(*index).chars().count(),
                Inner::Space | Inner::SoftLine => w += 1,
                Inner::SoftBreak | Inner::BreakPad(_) => {}
                Inner::HardLine => return None,
                Inner::Group(ds)
                | Inner::Indent(ds)
                | Inner::IndentBy(_, ds)
                | Inner::LeadIndent(ds)
                | Inner::IndentCols(_, ds)
                | Inner::Cont(ds)
                | Inner::Hang(ds)
                | Inner::Concat(ds) => stack.extend(ds.iter().rev()),
                Inner::Choice(a, _) => stack.push(a),
                Inner::Shared(d) => stack.push(d),
                Inner::Fill(items) => {
                    for (sep, c) in items.iter().rev() {
                        stack.push(c);
                        stack.push(sep);
                    }
                }
                Inner::AlignGroup(items, ..) => {
                    let rows = items.iter().filter_map(AlignItem::docs).count();
                    w += rows.saturating_sub(1);
                    for docs in items.iter().rev().filter_map(AlignItem::docs) {
                        stack.extend(docs.rev());
                    }
                }
                Inner::Unknown { first, last, .. } => w += self.raw_width(true, *first, *last),
                Inner::Verbatim { first, last, .. } | Inner::Opaque { first, last, .. } => {
                    w += self.raw_width(false, *first, *last)
                }
            }
        }
        Some(w)
    }
}

/// State of a trial print (`Printer::trial_fits`): it stops at the first
/// finished line that is too wide, because the branch cannot fit then.
struct Trial {
    width: usize,
    /// Where the trial's text starts in `out`, and the column there.
    start: usize,
    start_col: usize,
    /// Start of the line being written.
    line_from: usize,
    /// `out` holds no line break between `line_from` and here.
    scanned: usize,
    /// Blanks before `start` were removed: lines are only checked at the end.
    unchecked: bool,
    /// A line is too wide.
    failed: bool,
}

struct Printer<'c, 'a> {
    ctx: &'c Ctx<'a>,
    out: String,
    /// Set in a trial print.
    trial: Option<Trial>,
    /// Bytes of the real output before `out` (a trial printer holds only the
    /// current line).
    base: usize,
    /// The output grew beyond `Ctx::max_len`; printing stopped.
    overflow: bool,
    /// Characters on the current line (including pending indentation).
    col: usize,
    pending_indent: Option<usize>,
    /// An end-of-line comment was printed: the line must end before more
    /// content is written.
    pending_eol: bool,
    /// The current line holds only indentation (or nothing).
    line_empty: bool,
    /// The previous line is blank (to keep at most one blank line).
    prev_blank: bool,
    /// The leading comments of the next token are one level deeper
    /// (`Inner::LeadIndent`).
    lead_deeper: bool,
}

impl<'c, 'a> Printer<'c, 'a> {
    fn new(ctx: &'c Ctx<'a>) -> Printer<'c, 'a> {
        Printer {
            ctx,
            out: String::new(),
            trial: None,
            base: 0,
            overflow: false,
            col: 0,
            pending_indent: None,
            pending_eol: false,
            line_empty: true,
            prev_blank: true,
            lead_deeper: false,
        }
    }

    fn finish(mut self) -> String {
        // Comments anchored to the virtual end-of-file token.
        let eof = TokenIndex(self.ctx.tokens.tokens.len() as u32);
        self.leading_comments(eof, 0, true);
        while self.out.ends_with(BLANKS) {
            self.out.pop();
        }
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.out
    }

    /// Removes blanks at the end of the current line.
    fn trim_line_end(&mut self) {
        while self.out.ends_with(BLANKS) {
            self.out.pop();
        }
        if let Some(t) = &mut self.trial {
            t.scanned = t.scanned.min(self.out.len());
        }
    }

    /// Printing stopped: the output limit was reached, or a trial print found
    /// a line that is too wide.
    fn stopped(&mut self) -> bool {
        if self.base + self.out.len() > self.ctx.max_len {
            self.overflow = true;
        }
        if self.overflow {
            return true;
        }
        let Some(t) = &mut self.trial else {
            return false;
        };
        if t.failed {
            return true;
        }
        if t.unchecked || self.out.len() < t.line_from {
            t.unchecked = true;
            return false;
        }
        let out = &self.out;
        // Width of `out[a..b]` without trailing blanks, or 0 when it is
        // certainly narrow enough (bytes bound characters from above).
        let width_of = |t: &Trial, a: usize, b: usize| {
            let extra = if a == t.start { t.start_col } else { 0 };
            if b - a + extra <= t.width {
                return 0;
            }
            out[a..b].trim_end_matches(BLANKS).chars().count() + extra
        };
        let mut pos = t.scanned.max(t.line_from);
        while let Some(k) = out.as_bytes()[pos..].iter().position(|&c| c == b'\n') {
            let nl = pos + k;
            if width_of(t, t.line_from, nl) > t.width {
                t.failed = true;
                return true;
            }
            t.line_from = nl + 1;
            pos = nl + 1;
        }
        t.scanned = out.len();
        // The current line can only lose trailing blanks.
        if width_of(t, t.line_from, out.len()) > t.width {
            t.failed = true;
        }
        t.failed
    }

    fn newline(&mut self, indent: usize) {
        self.trim_line_end();
        self.prev_blank = self.line_empty;
        self.out.push('\n');
        self.col = 0;
        self.pending_indent = Some(indent);
        self.pending_eol = false;
        self.line_empty = true;
    }

    /// At least `n` blank lines before the next line (only `blank_line`
    /// limits them to one).
    fn blank_lines(&mut self, n: usize) {
        if self.out.is_empty() || self.trial.is_some() && self.out.is_empty() {
            return;
        }
        let indent = self.pending_indent.unwrap_or(0);
        if !self.line_empty {
            self.newline(indent);
        }
        let have = self
            .out
            .bytes()
            .rev()
            .take_while(|&c| c == b'\n')
            .count()
            .saturating_sub(1);
        for _ in have..n {
            self.out.push('\n');
        }
        self.prev_blank = true;
        self.pending_indent = Some(indent);
    }

    fn blank_line(&mut self) {
        if !self.prev_blank && !self.out.is_empty() {
            let indent = self.pending_indent.unwrap_or(0);
            if !self.line_empty {
                self.newline(indent);
            }
            self.out.push('\n');
            self.prev_blank = true;
            self.pending_indent = Some(indent);
        }
    }

    fn write(&mut self, text: &str, indent: usize) {
        if self.pending_eol {
            self.newline(indent);
        }
        if let Some(p) = self.pending_indent.take() {
            for _ in 0..p {
                self.out.push(' ');
            }
            self.col = p;
        }
        self.out.push_str(text);
        match text.rfind('\n') {
            Some(k) => self.col = text[k + 1..].chars().count(),
            None => self.col += text.chars().count(),
        }
        self.line_empty = false;
    }

    /// Column where the next character will be written.
    fn column(&self) -> usize {
        self.pending_indent.unwrap_or(self.col)
    }

    fn comment_text(&self, k: usize) -> std::borrow::Cow<'_, str> {
        let t = &self.ctx.tokens.trivia[k];
        let text = t.text(self.ctx.src);
        if self.ctx.opts.comment_style
            && !self.ctx.opts.identity
            && matches!(
                t.kind,
                TriviaKind::Comment {
                    syntax: CommentSyntax::AbapQuote,
                    ..
                }
            )
            && let Some(s) = origami_syntax::spaced_comment(text)
        {
            return std::borrow::Cow::Owned(s);
        }
        std::borrow::Cow::Borrowed(text)
    }

    /// Spaces in front of the end-of-line comment `k` that trails token `i`:
    /// as written (at least one) in the aligned style, else one.
    fn comment_gap(&self, i: TokenIndex, k: usize) -> usize {
        if !self.ctx.opts.comment_style {
            return 1;
        }
        let end = self.ctx.tokens.tokens[i.0 as usize].end as usize;
        let start = self.ctx.tokens.trivia[k].start as usize;
        match self.ctx.src.get(end..start) {
            Some(gap) if gap.bytes().all(|b| b == b' ' || b == b'\t') => gap.len().max(1),
            _ => 1,
        }
    }

    fn leading_comments(&mut self, i: TokenIndex, indent: usize, blank: bool) {
        let deeper = std::mem::take(&mut self.lead_deeper);
        // Never in front of a glued token (a statement glued to the period
        // of the one before stays on its line).
        if let Some(&n) = self.ctx.opts.forced_blanks.get(&(i.0 as usize))
            && !self.ctx.opts.identity
            && !self
                .ctx
                .tokens
                .tokens
                .get(i.0 as usize)
                .is_some_and(|t| t.glued)
        {
            self.blank_lines(n as usize);
        }
        let leading = &self.ctx.leading[i.0 as usize];
        // A token that starts a line stays in that line's column after its
        // comment lines, as it would without them.
        let indent = match self.pending_indent {
            Some(col) if !leading.is_empty() && self.line_empty && !self.pending_eol => col,
            _ => indent,
        };
        // The comment lines; the token keeps `indent`.
        let comment_indent = if deeper {
            indent + self.ctx.opts.indent as usize
        } else {
            indent
        };
        let overrides = &self.ctx.opts.comment_overrides;
        let width = self.ctx.opts.width as usize;
        let use_overrides = !overrides.is_empty()
            && leading.iter().all(|l| match overrides.get(&l.trivia) {
                Some(new) => {
                    let n = new.chars().count();
                    comment_indent + n <= width || n <= self.comment_text(l.trivia).chars().count()
                }
                None => true,
            });
        for (n, l) in leading.iter().enumerate() {
            if !self.line_empty || self.pending_eol {
                self.newline(comment_indent);
            }
            if blank && l.blank_before {
                self.blank_line();
            }
            let t = &self.ctx.tokens.trivia[l.trivia];
            let mut text = match overrides.get(&l.trivia) {
                Some(new) if use_overrides => new.clone(),
                _ => self.comment_text(l.trivia).to_string(),
            };
            if matches!(
                t.kind,
                TriviaKind::Comment {
                    syntax: CommentSyntax::AbapStar,
                    ..
                }
            ) {
                self.pending_indent = Some(0);
            } else {
                self.fit_line_limit(l.trivia, &mut text);
            }
            if deeper {
                self.pending_indent = Some(comment_indent);
            }
            self.write(&text, comment_indent);
            // The token (or the next comment) after the last comment line.
            self.newline(if n + 1 < leading.len() {
                comment_indent
            } else {
                indent
            });
        }
        let keep = blank
            || (!leading.is_empty()
                && self.ctx.opts.blank_after_comments.contains(&(i.0 as usize)));
        if keep
            && self
                .ctx
                .blank_before_token
                .get(i.0 as usize)
                .copied()
                .unwrap_or(false)
        {
            self.blank_line();
        }
    }

    /// An own-line comment `k` about to be written as `text` that would end
    /// past `PrintOptions::line_limit`, while its input line did not, keeps
    /// its input text and moves left until it fits (ABAP-COMMENT-003).
    fn fit_line_limit(&mut self, k: usize, text: &mut String) {
        let Some(limit) = self.ctx.opts.line_limit else {
            return;
        };
        let col = self.column();
        if col + text.chars().count() <= limit {
            return;
        }
        let src = self.ctx.src;
        let t = &self.ctx.tokens.trivia[k];
        let (a, b) = (t.start as usize, t.end as usize);
        let line_start = src[..a].rfind('\n').map_or(0, |p| p + 1);
        let line_end = src[b..].find('\n').map_or(src.len(), |p| b + p);
        if src[line_start..line_end].chars().count() > limit {
            return;
        }
        let own = t.text(src);
        self.pending_indent = Some(col.min(limit - own.chars().count()));
        *text = own.to_string();
        if self.trial.is_none() {
            self.ctx.spared.borrow_mut().insert(k);
        }
    }

    fn trailing_comments(&mut self, i: TokenIndex, indent: usize) {
        for &k in &self.ctx.trailing[i.0 as usize] {
            let placement = match self.ctx.tokens.trivia[k].kind {
                TriviaKind::Comment { placement, .. } => placement,
                _ => Placement::EndOfLine,
            };
            debug_assert_eq!(placement, Placement::EndOfLine);
            let text = self.comment_text(k).to_string();
            let gap = self.comment_gap(i, k);
            self.out.push_str(&" ".repeat(gap));
            self.col += gap;
            self.out.push_str(&text);
            self.col += text.chars().count();
            self.pending_eol = true;
        }
        let _ = indent;
    }

    fn token(&mut self, i: TokenIndex, indent: usize, blank: bool) {
        self.leading_comments(i, indent, blank);
        let text = self.ctx.token_text(i);
        self.write_token(&text, indent);
        self.trailing_comments(i, indent);
    }

    /// Writes token text. A token that starts with `*` is never written in
    /// column 1, where ABAP would read the line as a comment (INVARIANTS 4).
    fn write_token(&mut self, text: &str, indent: usize) {
        if self.pending_eol {
            self.newline(indent);
        }
        if text.starts_with('*') && self.column() == 0 {
            self.pending_indent = Some(1);
        }
        self.write(text, indent);
    }

    /// Unknown / verbatim / opaque text of a unit.
    fn raw(
        &mut self,
        kind: RawKind,
        first: TokenIndex,
        last: TokenIndex,
        indent: usize,
        blank: bool,
    ) {
        self.leading_comments(first, indent, blank);
        let ctx = self.ctx;
        let toks = &ctx.tokens.tokens;
        let src = ctx.src;
        if kind == RawKind::Opaque {
            let (a, b) = ctx.raw_range(first, last);
            // A unit that starts its line in the input and in the output also
            // keeps its indentation. Its other lines keep theirs, so moving
            // only the first line would change what the next run sees, and so
            // the next run's layout (idempotency).
            let line_start = src[..a].rfind('\n').map_or(0, |p| p + 1);
            let prefix = &src[line_start..a];
            if self.line_empty && !self.pending_eol && prefix.chars().all(|c| BLANKS.contains(&c)) {
                self.pending_indent = Some(0);
                self.write(&src[line_start..b], indent);
                self.trailing_comments(last, indent);
                return;
            }
            // Opaque text keeps its bytes, but a leading `*` never lands in
            // column 1 (INVARIANTS 4), where it would start a comment.
            if src[a..b].starts_with('*') && self.column() == 0 && self.line_empty {
                self.pending_indent = Some(1);
            }
            self.write(&src[a..b], indent);
            self.trailing_comments(last, indent);
            return;
        }
        // Original column of the first token, and where it lands now.
        let start = toks[first.0 as usize].start as usize;
        let line_start = src[..start].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let orig = visual_width(&src[line_start..start], ctx.opts.indent as usize);
        // A unit that starts with `*` never starts in column 1 (see
        // write_token); shift its other lines by the same amount.
        let star = ctx.token_text(first).starts_with('*');
        // Measured like `orig`: a tab already on this output line counts as
        // one indent step.
        let here = match self.pending_indent {
            Some(c) => c,
            None => {
                let ls = self.out.rfind('\n').map(|p| p + 1).unwrap_or(0);
                visual_width(&self.out[ls..], ctx.opts.indent as usize)
            }
        };
        let target = if star && here == 0 { 1 } else { here };
        let shift = target as isize - orig as isize;
        // Unknown units indent continuation lines at least one step. A unit
        // left as written keeps its relative indentation exactly: raising
        // some of its lines would change what the next run sees, and so the
        // next run's layout (idempotency).
        let min_indent = if kind == RawKind::Verbatim {
            0
        } else {
            ctx.opts.indent as usize
        };

        for k in first.0..=last.0 {
            let k = k as usize;
            // A token glued by the layout (ABAP-SPACE-001) loses the blanks
            // and line breaks before it.
            if k > first.0 as usize && !(kind == RawKind::Unknown && glued_gap(ctx, k)) {
                let gap = &src[toks[k - 1].end as usize..toks[k].start as usize];
                if gap.contains('\n') {
                    self.raw_gap(toks[k - 1].end as usize, gap, shift, min_indent, indent);
                } else if !gap.is_empty() {
                    // Same line: shrink blanks (unknown) or keep (verbatim).
                    if kind == RawKind::Unknown {
                        self.write(&same_line_gap(gap), indent);
                    } else {
                        self.write(gap, indent);
                    }
                }
            }
            let text = ctx.token_text(TokenIndex(k as u32));
            self.write_token(&text, indent);
        }
        self.trailing_comments(last, indent);
    }

    /// A gap with line breaks inside a raw unit: comments and line breaks are
    /// kept, every new line is shifted (`*` comments stay in column 1), and
    /// code continuation lines never land below `min_indent`.
    fn raw_gap(&mut self, from: usize, gap: &str, shift: isize, min_indent: usize, indent: usize) {
        // A block comment spanning lines keeps every byte: write the gap as
        // it is (INVARIANTS 3: only the first line of a block comment moves).
        // A one-line block comment, or `/*` in the text of a line comment, is
        // no reason to keep the gap.
        let trivia = &self.ctx.tokens.trivia;
        let at = trivia.partition_point(|t| (t.start as usize) < from);
        let to = from + gap.len();
        if trivia[at..]
            .iter()
            .take_while(|t| (t.start as usize) < to)
            .any(|t| {
                matches!(
                    t.kind,
                    TriviaKind::Comment {
                        syntax: CommentSyntax::CdsBlock,
                        ..
                    }
                ) && t.text(self.ctx.src).contains('\n')
            })
        {
            self.pending_eol = false;
            self.write(gap, indent);
            return;
        }
        let lines: Vec<&str> = gap.split('\n').collect();
        // Part before the first line break: end-of-line comment or spaces.
        // Only blanks are layout; other whitespace (vertical tab, form feed,
        // NBSP) belongs to the comment or token next to it.
        let head = lines[0].trim_matches(BLANKS);
        if !head.is_empty() {
            self.write(" ", indent);
            self.write(head, indent);
        }
        let last = lines.len() - 1;
        for (n, line) in lines.iter().enumerate().skip(1) {
            let trimmed = line.trim_start_matches(BLANKS);
            let orig = visual_width(
                &line[..line.len() - trimmed.len()],
                self.ctx.opts.indent as usize,
            );
            let is_star = trimmed.starts_with('*') && line.starts_with('*');
            let mut target = if is_star {
                0
            } else {
                (orig as isize + shift).max(0) as usize
            };
            if !is_star && n == last {
                target = target.max(min_indent);
            }
            // Write the line break and indentation.
            self.pending_eol = false;
            self.trim_line_end();
            self.out.push('\n');
            self.col = 0;
            self.line_empty = true;
            if n == last {
                self.pending_indent = Some(target);
            } else if trimmed.trim_end_matches(BLANKS).is_empty() {
                self.pending_indent = None;
            } else {
                self.pending_indent = Some(target);
                self.write(trimmed.trim_end_matches(BLANKS), indent);
            }
        }
    }

    fn fits(&self, next: Cmd, rest: &[Cmd], width: usize) -> bool {
        let mut remaining = width as isize - self.column() as isize;
        if self.pending_eol {
            return false;
        }
        let mut stack: Vec<(Mode, &Doc)> = vec![(Mode::Flat, next.doc)];
        let mut rest_idx = rest.len();
        loop {
            if remaining < 0 {
                return false;
            }
            let (mode, d) = match stack.pop() {
                Some(x) => x,
                None => {
                    if rest_idx == 0 {
                        return true;
                    }
                    rest_idx -= 1;
                    (rest[rest_idx].mode, rest[rest_idx].doc)
                }
            };
            match &d.0 {
                Inner::Token { index, .. } => {
                    remaining -= self.ctx.token_text(*index).chars().count() as isize;
                    if self.ctx.has_trailing_comments(*index) {
                        return remaining >= 0;
                    }
                }
                Inner::Space => remaining -= 1,
                Inner::SoftLine | Inner::SoftBreak if mode == Mode::Break => return true,
                Inner::SoftLine => remaining -= 1,
                Inner::SoftBreak => {}
                Inner::BreakPad(n) if mode == Mode::Break => remaining -= *n as isize,
                Inner::BreakPad(_) => {}
                Inner::HardLine => return true,
                Inner::Group(ds)
                | Inner::Indent(ds)
                | Inner::IndentBy(_, ds)
                | Inner::LeadIndent(ds)
                | Inner::IndentCols(_, ds)
                | Inner::Cont(ds)
                | Inner::Hang(ds)
                | Inner::Concat(ds) => {
                    let m = if matches!(d.0, Inner::Group(_))
                        && mode == Mode::Break
                        && rest_idx < rest.len()
                    {
                        // Groups after the measured one are measured flat only
                        // up to their first possible break.
                        Mode::Flat
                    } else {
                        mode
                    };
                    stack.extend(ds.iter().rev().map(|x| (m, x)));
                }
                Inner::Choice(a, _) => stack.push((mode, a)),
                Inner::Shared(d) => stack.push((mode, d)),
                Inner::Fill(items) => {
                    for (sep, c) in items.iter().rev() {
                        stack.push((mode, c));
                        stack.push((Mode::Flat, sep));
                    }
                }
                Inner::AlignGroup(items, ..) if mode == Mode::Flat => {
                    let mut flat: Vec<(Mode, &Doc)> = Vec::new();
                    for (k, docs) in items.iter().filter_map(AlignItem::docs).enumerate() {
                        if k > 0 {
                            flat.push((Mode::Flat, &SPACE_DOC));
                        }
                        flat.extend(docs.map(|d| (Mode::Flat, d)));
                    }
                    stack.extend(flat.into_iter().rev());
                }
                Inner::AlignGroup(..) => return remaining >= 0,
                Inner::Unknown { first, last, .. }
                | Inner::Verbatim { first, last, .. }
                | Inner::Opaque { first, last, .. } => {
                    let unknown = matches!(d.0, Inner::Unknown { .. });
                    let (a, b) = self.ctx.raw_range(*first, *last);
                    let multiline = self.ctx.src[a..b].contains('\n');
                    remaining -= self.ctx.raw_width(unknown, *first, *last) as isize;
                    if multiline {
                        return remaining >= 0;
                    }
                }
            }
        }
    }

    fn run<'d>(&mut self, mut stack: Vec<Cmd<'d>>) {
        let width = self.ctx.opts.width as usize;
        let step = self.ctx.opts.indent as usize;
        while let Some(cmd) = stack.pop() {
            if self.stopped() {
                return;
            }
            let Cmd { indent, mode, doc } = cmd;
            match &doc.0 {
                Inner::Token { index, blank } => self.token(*index, indent, *blank),
                Inner::Space => {
                    if self.pending_eol {
                        self.newline(indent);
                    } else {
                        self.write(" ", indent);
                    }
                }
                Inner::SoftLine | Inner::SoftBreak if mode == Mode::Break || self.pending_eol => {
                    self.newline(indent)
                }
                Inner::SoftLine => self.write(" ", indent),
                Inner::SoftBreak => {}
                Inner::BreakPad(n) if mode == Mode::Break => self.pad(*n as usize, indent),
                Inner::BreakPad(_) => {}
                Inner::HardLine => self.newline(indent),
                Inner::Concat(ds) => {
                    stack.extend(ds.iter().rev().map(|d| Cmd {
                        indent,
                        mode,
                        doc: d,
                    }));
                }
                Inner::Indent(ds) => {
                    stack.extend(ds.iter().rev().map(|d| Cmd {
                        indent: indent + step,
                        mode,
                        doc: d,
                    }));
                }
                Inner::IndentBy(n, ds) => {
                    stack.extend(ds.iter().rev().map(|d| Cmd {
                        indent: indent + step * *n as usize,
                        mode,
                        doc: d,
                    }));
                }
                Inner::IndentCols(n, ds) => {
                    stack.extend(ds.iter().rev().map(|d| Cmd {
                        indent: indent + *n as usize,
                        mode,
                        doc: d,
                    }));
                }
                Inner::LeadIndent(ds) => {
                    self.lead_deeper = true;
                    stack.extend(ds.iter().rev().map(|d| Cmd {
                        indent,
                        mode,
                        doc: d,
                    }));
                }
                Inner::Cont(ds) => {
                    stack.extend(ds.iter().rev().map(|d| Cmd {
                        indent: indent + step * self.ctx.opts.cont_levels as usize,
                        mode,
                        doc: d,
                    }));
                }
                Inner::Hang(ds) => {
                    let col = self.column();
                    stack.extend(ds.iter().rev().map(|d| Cmd {
                        indent: col,
                        mode,
                        doc: d,
                    }));
                }
                Inner::Group(ds) => {
                    let m = if mode == Mode::Flat
                        || (!self.ctx.is_forced(doc)
                            && self.fits(
                                Cmd {
                                    indent,
                                    mode: Mode::Flat,
                                    doc,
                                },
                                &stack,
                                width,
                            )) {
                        Mode::Flat
                    } else {
                        Mode::Break
                    };
                    stack.extend(ds.iter().rev().map(|d| Cmd {
                        indent,
                        mode: m,
                        doc: d,
                    }));
                }
                Inner::Shared(d) => stack.push(Cmd {
                    indent,
                    mode,
                    doc: d,
                }),
                Inner::Choice(a, b) => {
                    let pick = if self.trial_fits(a, indent, mode, width) {
                        a
                    } else {
                        b
                    };
                    stack.push(Cmd {
                        indent,
                        mode,
                        doc: pick,
                    });
                }
                Inner::Fill(items) => {
                    // Decide each separator when it is reached.
                    let mut cmds: Vec<Cmd> = Vec::new();
                    for (sep, c) in items {
                        cmds.push(Cmd {
                            indent,
                            mode,
                            doc: sep,
                        });
                        cmds.push(Cmd {
                            indent,
                            mode,
                            doc: c,
                        });
                    }
                    let mut k = 0;
                    while k < cmds.len() && !self.stopped() {
                        let sep = cmds[k];
                        let content = cmds[k + 1];
                        // The last item shares its line with what follows the
                        // fill (for example the statement's period).
                        let rest: &[Cmd] = if k + 2 >= cmds.len() { &stack } else { &[] };
                        let flat_ok = mode == Mode::Flat
                            || (!self.pending_eol
                                && self.fits(
                                    Cmd {
                                        indent,
                                        mode: Mode::Flat,
                                        doc: content.doc,
                                    },
                                    rest,
                                    width - 1,
                                ));
                        let sep_mode = if flat_ok { Mode::Flat } else { Mode::Break };
                        self.run(vec![Cmd {
                            indent: sep.indent,
                            mode: sep_mode,
                            doc: sep.doc,
                        }]);
                        self.run(vec![content]);
                        k += 2;
                    }
                }
                Inner::AlignGroup(items, ..) if mode == Mode::Flat => {
                    let mut flat: Vec<Cmd> = Vec::new();
                    for (k, docs) in items.iter().filter_map(AlignItem::docs).enumerate() {
                        if k > 0 {
                            flat.push(Cmd {
                                indent,
                                mode,
                                doc: &SPACE_DOC,
                            });
                        }
                        flat.extend(docs.map(|d| Cmd {
                            indent,
                            mode,
                            doc: d,
                        }));
                    }
                    stack.extend(flat.into_iter().rev());
                }
                Inner::AlignGroup(items, fit, _) => self.align_group(items, *fit, indent, width),
                Inner::Unknown { first, last, blank } => {
                    self.raw(RawKind::Unknown, *first, *last, indent, *blank)
                }
                Inner::Verbatim { first, last, blank } => {
                    self.raw(RawKind::Verbatim, *first, *last, indent, *blank)
                }
                Inner::Opaque { first, last, blank } => {
                    self.raw(RawKind::Opaque, *first, *last, indent, *blank)
                }
            }
        }
    }

    /// Prints `doc` into a copy of this printer and checks every line. The
    /// copy starts with the current line only, and the answer is remembered
    /// for the same doc and state.
    fn trial_fits(&mut self, doc: &Doc, indent: usize, mode: Mode, width: usize) -> bool {
        let line_start = self.out.rfind('\n').unwrap_or(0);
        let line = &self.out[self.out.rfind('\n').map_or(0, |p| p + 1)..];
        let key = TrialKey {
            doc,
            indent,
            mode,
            width,
            col: self.col,
            pending_indent: self.pending_indent,
            pending_eol: self.pending_eol,
            line_empty: self.line_empty,
            prev_blank: self.prev_blank,
            out_empty: self.out.is_empty(),
            line_width: visual_width(line, self.ctx.opts.indent as usize),
            trailing_blanks: line[line.trim_end_matches(BLANKS).len()..].to_string(),
        };
        if let Some(&fits) = self.ctx.trials.borrow().get(&key) {
            return fits;
        }

        let seed = self.out[line_start..].to_string();
        let start = seed.len();
        // Characters already on the line; pending indentation is written by
        // the trial itself, so it must not be counted here too.
        let start_col = self.col;
        let mut trial = Printer {
            ctx: self.ctx,
            out: seed,
            trial: Some(Trial {
                width,
                start,
                start_col,
                line_from: start,
                scanned: start,
                unchecked: false,
                failed: false,
            }),
            base: self.base + line_start,
            overflow: false,
            col: self.col,
            pending_indent: self.pending_indent,
            pending_eol: self.pending_eol,
            line_empty: self.line_empty,
            prev_blank: self.prev_blank,
            lead_deeper: false,
        };
        trial.run(vec![Cmd { indent, mode, doc }]);
        if trial.overflow {
            // The branch alone exceeds the output limit: stop printing.
            self.overflow = true;
            return false;
        }
        // A trial that stopped early has a line that is too wide.
        let failed = trial.trial.as_ref().is_some_and(|t| t.failed);
        let fits = !failed
            && trial.out[start..].split('\n').enumerate().all(|(k, line)| {
                let w = line.chars().count();
                (if k == 0 { start_col + w } else { w }) <= width
            });
        self.ctx.trials.borrow_mut().insert(key, fits);
        fits
    }

    /// Indent for a cell of an aligned row: a cell whose first token has
    /// comment lines in front of it starts at the current column, so the
    /// comments and the token stay in the cell's column.
    fn cell_indent(&self, cell: &[Doc], col: usize) -> usize {
        match cell.iter().find_map(first_token) {
            Some(i) if self.ctx.has_leading_comments(i) => self.column(),
            _ => col,
        }
    }

    /// Padding of an aligned cell. At the start of a line it becomes
    /// indentation, so comment lines in front of the next token do not
    /// leave a line of blanks behind.
    fn pad(&mut self, n: usize, indent: usize) {
        match self.pending_indent {
            Some(p) if self.line_empty && !self.pending_eol => self.pending_indent = Some(p + n),
            _ => self.write(&" ".repeat(n), indent),
        }
    }

    fn align_group(&mut self, items: &[AlignItem], fit: AlignFit, indent: usize, width: usize) {
        let col = self.column().max(indent);
        // Split into segments at `Break`; each segment is one block for the
        // lead columns.
        let mut segments: Vec<(usize, Vec<&AlignItem>)> = vec![(0, Vec::new())];
        for it in items {
            match it {
                AlignItem::Break => segments.push((segments.len(), Vec::new())),
                _ => segments.last_mut().unwrap().1.push(it),
            }
        }
        fn rows_of<'x>(seg: &[&'x AlignItem]) -> Vec<&'x crate::ir::AlignRow> {
            seg.iter()
                .filter_map(|it| match it {
                    AlignItem::Row(r) => Some(r),
                    _ => None,
                })
                .collect()
        }
        let mut blocks: Vec<Vec<&crate::ir::AlignRow>> = vec![Vec::new(); segments.len()];
        for (block, seg) in &segments {
            blocks[*block].extend(rows_of(seg));
        }
        let mut first_row = true;
        for (block, seg) in segments.iter().filter(|s| !s.1.is_empty()) {
            let rows = rows_of(seg);
            let block_rows = &blocks[*block];
            let heads: Vec<Option<usize>> =
                rows.iter().map(|r| self.ctx.flat_width(&r.head)).collect();
            // A row without lead columns and without a tail (`OTHERS` in
            // `EXCEPTIONS`) has no anchor and blocks alignment; with lead
            // columns, a row may end early (a declaration without `VALUE`).
            let open_row = |r: &crate::ir::AlignRow| r.tail.is_empty() && !r.lead.is_empty();
            let anchor_glued = rows.iter().any(|r| {
                !open_row(r) && !matches!(r.tail.first().map(|d| &d.0), Some(Inner::Space))
            });
            // Lead columns: each as wide as its widest cell among the rows
            // of the block with content after it.
            let n_cols = block_rows.iter().map(|r| r.lead.len()).max().unwrap_or(0);
            let after = |r: &crate::ir::AlignRow, c: usize| {
                r.lead.iter().skip(c + 1).any(|x| !x.is_empty())
                    || !r.head.is_empty()
                    || !r.tail.is_empty()
            };
            let mut cells_flat = true;
            let mut col_w = vec![0usize; n_cols];
            for r in block_rows {
                for (c, cell) in r.lead.iter().enumerate() {
                    match self.ctx.flat_width(cell) {
                        Some(w) if after(r, c) => col_w[c] = col_w[c].max(w),
                        Some(_) => {}
                        None => cells_flat = false,
                    }
                }
            }
            // Width of a row's lead part when aligned, and unaligned.
            let lead_w = |r: &crate::ir::AlignRow, aligned: bool| -> usize {
                r.lead
                    .iter()
                    .enumerate()
                    .map(|(c, cell)| {
                        let w = self.ctx.flat_width(cell).unwrap_or(0);
                        if aligned && after(r, c) { col_w[c] } else { w }
                    })
                    .sum()
            };
            // Heads are padded to the widest head, and anchors of different
            // widths are right-aligned in a column as wide as the widest.
            let max = rows
                .iter()
                .zip(&heads)
                .filter(|(r, _)| !open_row(r))
                .filter_map(|(_, h)| *h)
                .max()
                .unwrap_or(0);
            let max_anchor = rows.iter().map(|r| r.anchor).max().unwrap_or(0);
            let pad = |r: &crate::ir::AlignRow, h: usize| max - h + max_anchor - r.anchor;
            let overflow = !cells_flat
                || rows.iter().zip(&heads).any(|(r, h)| {
                    let (la, lu) = (lead_w(r, true), lead_w(r, false));
                    match (h, fit) {
                        (None, _) => true,
                        _ if open_row(r) => col + la + h.unwrap_or(0) > width,
                        // Only rows whose tail can be flat count for overflow; a
                        // tail that breaks anyway (a nested multi-line
                        // expression) does not block alignment.
                        (Some(h), AlignFit::FirstLine) => {
                            col + la + h + pad(r, *h) + self.ctx.min_first_line(&r.tail) > width
                        }
                        // A row that fits on one line must still fit; otherwise
                        // its first line must still fit if it did.
                        (Some(h), AlignFit::KeepLines) => {
                            let line = match self.ctx.flat_width(&r.tail) {
                                Some(t) if col + lu + h + t <= width => t,
                                _ => self.ctx.min_first_line(&r.tail),
                            };
                            col + lu + h + line <= width && col + la + h + pad(r, *h) + line > width
                        }
                    }
                });
            let aligned = !anchor_glued && !overflow;
            for it in seg {
                if self.stopped() {
                    return;
                }
                if !first_row {
                    self.newline(col);
                }
                first_row = false;
                let r = match it {
                    AlignItem::Row(r) => r,
                    AlignItem::Line(docs) => {
                        self.run(
                            docs.iter()
                                .rev()
                                .map(|d| Cmd {
                                    indent: col,
                                    mode: Mode::Break,
                                    doc: d,
                                })
                                .collect(),
                        );
                        continue;
                    }
                    AlignItem::Break => continue,
                };
                for (c, cell) in r.lead.iter().enumerate() {
                    let at = self.cell_indent(cell, col);
                    self.run(
                        cell.iter()
                            .rev()
                            .map(|d| Cmd {
                                indent: at,
                                mode: Mode::Flat,
                                doc: d,
                            })
                            .collect(),
                    );
                    if aligned && after(r, c) {
                        let w = self.ctx.flat_width(cell).unwrap_or(0);
                        self.pad(col_w[c] - w, col);
                    }
                }
                let at = self.cell_indent(&r.head, col);
                self.run(
                    r.head
                        .iter()
                        .rev()
                        .map(|d| Cmd {
                            indent: at,
                            mode: Mode::Flat,
                            doc: d,
                        })
                        .collect(),
                );
                let mut tail: &[Doc] = &r.tail;
                if aligned && !r.tail.is_empty() {
                    let h = self.ctx.flat_width(&r.head).unwrap_or(0);
                    // Without any head in the segment, the last lead cell's
                    // space already separates the tail.
                    let gap =
                        usize::from(!(max == 0 && h == 0 && r.lead.iter().any(|c| !c.is_empty())));
                    let spaces = " ".repeat(pad(r, h) + gap);
                    self.write(&spaces, col);
                    tail = &r.tail[1..];
                }
                self.run(
                    tail.iter()
                        .rev()
                        .map(|d| Cmd {
                            indent: col,
                            mode: Mode::Break,
                            doc: d,
                        })
                        .collect(),
                );
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RawKind {
    Unknown,
    Verbatim,
    Opaque,
}

/// Token `k` is glued, but the source has blanks or line breaks (and nothing
/// else) before it: the language crate glued it.
fn glued_gap(ctx: &Ctx, k: usize) -> bool {
    let toks = &ctx.tokens.tokens;
    toks[k].glued
        && ctx.src[toks[k - 1].end as usize..toks[k].start as usize]
            .chars()
            .all(|c| BLANKS.contains(&c) || c == '\n')
}

/// Width of leading indentation; a tab counts as one indent step.
fn visual_width(s: &str, step: usize) -> usize {
    s.chars().map(|c| if c == '\t' { step } else { 1 }).sum()
}

/// A same-line gap inside an unknown unit as printed: runs of blanks shrink to
/// one space; a block comment inside the gap is kept.
fn same_line_gap(gap: &str) -> String {
    let inner = gap.trim_matches(BLANKS);
    if inner.is_empty() {
        return if gap.is_empty() {
            String::new()
        } else {
            " ".into()
        };
    }
    let mut out = String::new();
    if gap.starts_with(BLANKS) {
        out.push(' ');
    }
    out.push_str(inner);
    if gap.ends_with(BLANKS) {
        out.push(' ');
    }
    out
}

/// The first token of a doc, if it has one.
fn first_token(doc: &Doc) -> Option<TokenIndex> {
    match &doc.0 {
        Inner::Token { index, .. } => Some(*index),
        Inner::Unknown { first, .. }
        | Inner::Verbatim { first, .. }
        | Inner::Opaque { first, .. } => Some(*first),
        Inner::Group(ds)
        | Inner::Indent(ds)
        | Inner::Cont(ds)
        | Inner::Hang(ds)
        | Inner::Concat(ds)
        | Inner::LeadIndent(ds)
        | Inner::IndentBy(_, ds)
        | Inner::IndentCols(_, ds) => ds.iter().find_map(first_token),
        Inner::Choice(a, _) => first_token(a),
        Inner::Shared(d) => first_token(d),
        Inner::Fill(items) => items
            .iter()
            .find_map(|(s, c)| first_token(s).or_else(|| first_token(c))),
        Inner::AlignGroup(items, ..) => items
            .iter()
            .filter_map(AlignItem::docs)
            .find_map(|mut docs| docs.find_map(first_token)),
        Inner::Space
        | Inner::SoftLine
        | Inner::SoftBreak
        | Inner::BreakPad(_)
        | Inner::HardLine => None,
    }
}

/// The last token of a doc, if it has one.
fn last_token(doc: &Doc) -> Option<TokenIndex> {
    match &doc.0 {
        Inner::Token { index, .. } => Some(*index),
        Inner::Unknown { last, .. } | Inner::Verbatim { last, .. } | Inner::Opaque { last, .. } => {
            Some(*last)
        }
        Inner::Group(ds)
        | Inner::Indent(ds)
        | Inner::IndentBy(_, ds)
        | Inner::LeadIndent(ds)
        | Inner::IndentCols(_, ds)
        | Inner::Cont(ds)
        | Inner::Hang(ds)
        | Inner::Concat(ds) => ds.iter().rev().find_map(last_token),
        Inner::Choice(a, _) => last_token(a),
        Inner::Shared(d) => last_token(d),
        Inner::Fill(items) => items.iter().rev().find_map(|(_, c)| last_token(c)),
        Inner::AlignGroup(items, ..) => items
            .iter()
            .rev()
            .filter_map(AlignItem::docs)
            .find_map(|docs| docs.rev().find_map(last_token)),
        Inner::Space
        | Inner::SoftLine
        | Inner::SoftBreak
        | Inner::BreakPad(_)
        | Inner::HardLine => None,
    }
}

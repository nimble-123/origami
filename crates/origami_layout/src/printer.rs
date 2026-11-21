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
//!   widest head of its segment; a segment falls back to single spaces when
//!   padding would overflow or an anchor is glued.
//! * Comments are printed with their token. Own-line comments go on their own
//!   lines at the token's indent (`*` comments in column 1); end-of-line
//!   comments follow their token after one space and end the line.
//! * Indentation is written lazily, so lines never end in spaces.
//!
//! In identity mode the printer returns the source text unchanged (M0).

use crate::ir::{AlignItem, Doc, Inner};
use origami_syntax::{
    Case, CommentSyntax, Placement, SourceText, TokenIndex, TokenStream, TriviaKind, TriviaPosition,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintOptions {
    pub width: u16,
    pub indent: u8,
    /// `None` leaves every token as written (CDS family).
    pub keyword_case: Option<Case>,
    /// Return the source text unchanged.
    pub identity: bool,
}

pub fn print(doc: &Doc, src: &SourceText, tokens: &TokenStream, opts: &PrintOptions) -> String {
    if opts.identity {
        return src.text().to_string();
    }
    let ctx = Ctx::new(src.text(), tokens, opts);
    let mut p = Printer::new(&ctx);
    p.run(vec![Cmd {
        indent: 0,
        mode: Mode::Break,
        doc,
    }]);
    p.finish()
}

/// A space, for joining aligned rows printed flat.
static SPACE_DOC: Doc = Doc(Inner::Space);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

impl<'a> Ctx<'a> {
    fn new(src: &'a str, tokens: &'a TokenStream, opts: &'a PrintOptions) -> Ctx<'a> {
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
        }
    }

    fn token_text(&self, i: TokenIndex) -> String {
        let t = &self.tokens.tokens[i.0 as usize];
        let text = t.text(self.src);
        match (t.keyword, self.opts.keyword_case) {
            (true, Some(Case::Upper)) => text.to_ascii_uppercase(),
            (true, Some(Case::Lower)) => text.to_ascii_lowercase(),
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
        let mut w = 0;
        for k in first.0..=last.0 {
            let t = &self.tokens.tokens[k as usize];
            if k > first.0 && !t.glued {
                w += 1;
            }
            w += self.token_text(TokenIndex(k)).chars().count();
        }
        w
    }

    /// Width of the doc up to its first possible line break (the shortest
    /// first line it can be printed with).
    fn min_first_line(&self, doc: &Doc) -> usize {
        let mut w = 0usize;
        let mut stack = vec![doc];
        while let Some(d) = stack.pop() {
            match &d.0 {
                Inner::Token { index, .. } => {
                    w += self.token_text(*index).chars().count();
                    if self.has_trailing_comments(*index) {
                        return w;
                    }
                }
                Inner::Space => w += 1,
                Inner::SoftLine | Inner::SoftBreak | Inner::HardLine => return w,
                Inner::Group(ds) | Inner::Indent(ds) | Inner::Hang(ds) | Inner::Concat(ds) => {
                    stack.extend(ds.iter().rev())
                }
                Inner::Choice(a, _) => stack.push(a),
                Inner::Fill(items) => {
                    for (sep, c) in items.iter().rev() {
                        stack.push(c);
                        stack.push(sep);
                    }
                }
                Inner::AlignGroup(items) => {
                    if let Some(AlignItem::Row(r)) =
                        items.iter().find(|i| matches!(i, AlignItem::Row(_)))
                    {
                        stack.extend(r.tail.iter().rev());
                        stack.extend(r.head.iter().rev());
                    }
                }
                Inner::Unknown { first, last, .. } => {
                    let (a, b) = self.raw_range(*first, *last);
                    if self.src[a..b].contains('\n') {
                        let line = self.src[a..b].split('\n').next().unwrap_or("");
                        return w + line.chars().count();
                    }
                    w += self.raw_width(true, *first, *last);
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
    fn forced(&self, doc: &Doc, first: &mut bool) -> bool {
        match &doc.0 {
            Inner::HardLine => true,
            Inner::Token { index, .. } => {
                let lead = !*first && self.has_leading_comments(*index);
                *first = false;
                lead || self.has_trailing_comments(*index)
            }
            Inner::Unknown { first: f, last, .. }
            | Inner::Verbatim { first: f, last, .. }
            | Inner::Opaque { first: f, last, .. } => {
                let lead = !*first && self.has_leading_comments(*f);
                *first = false;
                let (a, b) = self.raw_range(*f, *last);
                lead || self.src[a..b].contains('\n') || self.has_trailing_comments(*last)
            }
            Inner::AlignGroup(items) => items.iter().any(|it| match it {
                AlignItem::Row(r) => r
                    .head
                    .iter()
                    .chain(r.tail.iter())
                    .any(|d| self.forced(d, first)),
                AlignItem::Break => false,
            }),
            Inner::Group(ds) | Inner::Indent(ds) | Inner::Hang(ds) | Inner::Concat(ds) => {
                ds.iter().any(|d| self.forced(d, first))
            }
            Inner::Choice(a, _) => self.forced(a, first),
            Inner::Fill(items) => items.iter().any(|(_, c)| self.forced(c, first)),
            Inner::Space | Inner::SoftLine | Inner::SoftBreak => false,
        }
    }

    fn is_forced(&self, doc: &Doc) -> bool {
        self.forced(doc, &mut true)
    }

    /// Width of the doc printed flat, or `None` if it cannot be flat.
    fn flat_width(&self, doc: &Doc) -> Option<usize> {
        if self.is_forced(doc) {
            return None;
        }
        let mut w = 0usize;
        let mut stack = vec![doc];
        while let Some(d) = stack.pop() {
            match &d.0 {
                Inner::Token { index, .. } => w += self.token_text(*index).chars().count(),
                Inner::Space | Inner::SoftLine => w += 1,
                Inner::SoftBreak => {}
                Inner::HardLine => return None,
                Inner::Group(ds) | Inner::Indent(ds) | Inner::Hang(ds) | Inner::Concat(ds) => {
                    stack.extend(ds.iter().rev())
                }
                Inner::Choice(a, _) => stack.push(a),
                Inner::Fill(items) => {
                    for (sep, c) in items.iter().rev() {
                        stack.push(c);
                        stack.push(sep);
                    }
                }
                Inner::AlignGroup(items) => {
                    let rows = items
                        .iter()
                        .filter(|i| matches!(i, AlignItem::Row(_)))
                        .count();
                    w += rows.saturating_sub(1);
                    for it in items {
                        if let AlignItem::Row(r) = it {
                            stack.extend(r.tail.iter().rev());
                            stack.extend(r.head.iter().rev());
                        }
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

#[derive(Clone)]
struct Printer<'c, 'a> {
    ctx: &'c Ctx<'a>,
    out: String,
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
}

impl<'c, 'a> Printer<'c, 'a> {
    fn new(ctx: &'c Ctx<'a>) -> Printer<'c, 'a> {
        Printer {
            ctx,
            out: String::new(),
            col: 0,
            pending_indent: None,
            pending_eol: false,
            line_empty: true,
            prev_blank: true,
        }
    }

    fn finish(mut self) -> String {
        // Comments anchored to the virtual end-of-file token.
        let eof = TokenIndex(self.ctx.tokens.tokens.len() as u32);
        self.leading_comments(eof, 0, true);
        while self.out.ends_with(' ') {
            self.out.pop();
        }
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.out
    }

    fn newline(&mut self, indent: usize) {
        while self.out.ends_with(' ') {
            self.out.pop();
        }
        self.prev_blank = self.line_empty;
        self.out.push('\n');
        self.col = 0;
        self.pending_indent = Some(indent);
        self.pending_eol = false;
        self.line_empty = true;
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

    fn comment_text(&self, k: usize) -> &str {
        self.ctx.tokens.trivia[k].text(self.ctx.src)
    }

    fn leading_comments(&mut self, i: TokenIndex, indent: usize, blank: bool) {
        let leading = &self.ctx.leading[i.0 as usize];
        for l in leading {
            if !self.line_empty || self.pending_eol {
                self.newline(indent);
            }
            if blank && l.blank_before {
                self.blank_line();
            }
            let t = &self.ctx.tokens.trivia[l.trivia];
            let text = self.comment_text(l.trivia).to_string();
            if matches!(
                t.kind,
                TriviaKind::Comment {
                    syntax: CommentSyntax::AbapStar,
                    ..
                }
            ) {
                self.pending_indent = Some(0);
            }
            self.write(&text, indent);
            self.newline(indent);
        }
        if blank
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

    fn trailing_comments(&mut self, i: TokenIndex, indent: usize) {
        for &k in &self.ctx.trailing[i.0 as usize] {
            let placement = match self.ctx.tokens.trivia[k].kind {
                TriviaKind::Comment { placement, .. } => placement,
                _ => Placement::EndOfLine,
            };
            debug_assert_eq!(placement, Placement::EndOfLine);
            let text = self.comment_text(k).to_string();
            self.out.push(' ');
            self.col += 1;
            self.out.push_str(&text);
            self.col += text.chars().count();
            self.pending_eol = true;
        }
        let _ = indent;
    }

    fn token(&mut self, i: TokenIndex, indent: usize, blank: bool) {
        self.leading_comments(i, indent, blank);
        let text = self.ctx.token_text(i);
        self.write(&text, indent);
        self.trailing_comments(i, indent);
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
            self.write(&src[a..b], indent);
            self.trailing_comments(last, indent);
            return;
        }
        // Original column of the first token, and where it lands now.
        let start = toks[first.0 as usize].start as usize;
        let line_start = src[..start].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let orig = visual_width(&src[line_start..start], ctx.opts.indent as usize);
        let target = self.column();
        let shift = target as isize - orig as isize;
        let min_indent = ctx.opts.indent as usize;

        for k in first.0..=last.0 {
            let k = k as usize;
            if k > first.0 as usize {
                let gap = &src[toks[k - 1].end as usize..toks[k].start as usize];
                if gap.contains('\n') {
                    self.raw_gap(gap, shift, min_indent, indent);
                } else if !gap.is_empty() {
                    // Same line: shrink spaces (unknown) or keep (verbatim).
                    let text = if kind == RawKind::Unknown { " " } else { gap };
                    self.write(text, indent);
                }
            }
            let text = ctx.token_text(TokenIndex(k as u32));
            self.write(&text, indent);
        }
        self.trailing_comments(last, indent);
    }

    /// A gap with line breaks inside a raw unit: comments and line breaks are
    /// kept, every new line is shifted (`*` comments stay in column 1), and
    /// code continuation lines never land below `min_indent`.
    fn raw_gap(&mut self, gap: &str, shift: isize, min_indent: usize, indent: usize) {
        let lines: Vec<&str> = gap.split('\n').collect();
        // Part before the first line break: end-of-line comment or spaces.
        let head = lines[0].trim();
        if !head.is_empty() {
            self.write(" ", indent);
            self.write(head, indent);
        }
        let last = lines.len() - 1;
        for (n, line) in lines.iter().enumerate().skip(1) {
            let trimmed = line.trim_start_matches([' ', '\t', '\r']);
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
            while self.out.ends_with(' ') {
                self.out.pop();
            }
            self.out.push('\n');
            self.col = 0;
            self.line_empty = true;
            if n == last {
                self.pending_indent = Some(target);
            } else if trimmed.trim().is_empty() {
                self.pending_indent = None;
            } else {
                self.pending_indent = Some(target);
                self.write(trimmed.trim_end(), indent);
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
                Inner::HardLine => return true,
                Inner::Group(ds) | Inner::Indent(ds) | Inner::Hang(ds) | Inner::Concat(ds) => {
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
                Inner::Fill(items) => {
                    for (sep, c) in items.iter().rev() {
                        stack.push((mode, c));
                        stack.push((Mode::Flat, sep));
                    }
                }
                Inner::AlignGroup(items) if mode == Mode::Flat => {
                    let mut flat: Vec<(Mode, &Doc)> = Vec::new();
                    for (k, it) in items
                        .iter()
                        .filter_map(|i| match i {
                            AlignItem::Row(r) => Some(r),
                            AlignItem::Break => None,
                        })
                        .enumerate()
                    {
                        if k > 0 {
                            flat.push((Mode::Flat, &SPACE_DOC));
                        }
                        flat.extend(
                            it.head
                                .iter()
                                .chain(it.tail.iter())
                                .map(|d| (Mode::Flat, d)),
                        );
                    }
                    stack.extend(flat.into_iter().rev());
                }
                Inner::AlignGroup(_) => return remaining >= 0,
                Inner::Unknown { first, last, .. }
                | Inner::Verbatim { first, last, .. }
                | Inner::Opaque { first, last, .. } => {
                    let unknown = matches!(d.0, Inner::Unknown { .. });
                    let (a, b) = self.ctx.raw_range(*first, *last);
                    let multiline = self.ctx.src[a..b].contains('\n');
                    remaining -= self.ctx.raw_width(unknown && !multiline, *first, *last) as isize;
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
                    while k < cmds.len() {
                        let sep = cmds[k];
                        let content = cmds[k + 1];
                        let flat_ok = mode == Mode::Flat
                            || (!self.pending_eol
                                && self.fits(
                                    Cmd {
                                        indent,
                                        mode: Mode::Flat,
                                        doc: content.doc,
                                    },
                                    &[],
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
                Inner::AlignGroup(items) if mode == Mode::Flat => {
                    let mut flat: Vec<Cmd> = Vec::new();
                    let rows = items.iter().filter_map(|i| match i {
                        AlignItem::Row(r) => Some(r),
                        AlignItem::Break => None,
                    });
                    for (k, r) in rows.enumerate() {
                        if k > 0 {
                            flat.push(Cmd {
                                indent,
                                mode,
                                doc: &SPACE_DOC,
                            });
                        }
                        flat.extend(r.head.iter().chain(r.tail.iter()).map(|d| Cmd {
                            indent,
                            mode,
                            doc: d,
                        }));
                    }
                    stack.extend(flat.into_iter().rev());
                }
                Inner::AlignGroup(items) => self.align_group(items, indent, width),
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

    /// Prints `doc` into a copy of this printer and checks every line.
    fn trial_fits(&self, doc: &Doc, indent: usize, mode: Mode, width: usize) -> bool {
        let mut trial = self.clone();
        let start = trial.out.len();
        let start_col = trial.column();
        trial.run(vec![Cmd { indent, mode, doc }]);
        let text = &trial.out[start..];
        let mut first = true;
        for line in text.split('\n') {
            let w = if first {
                start_col + line.chars().count()
            } else {
                line.chars().count()
            };
            first = false;
            if w > width {
                return false;
            }
        }
        true
    }

    fn align_group(&mut self, items: &[AlignItem], indent: usize, width: usize) {
        let col = self.column().max(indent);
        // Split into segments at `Break`.
        let mut segments: Vec<Vec<&crate::ir::AlignRow>> = vec![Vec::new()];
        for it in items {
            match it {
                AlignItem::Row(r) => segments.last_mut().unwrap().push(r),
                AlignItem::Break => segments.push(Vec::new()),
            }
        }
        let mut first_row = true;
        for seg in segments.iter().filter(|s| !s.is_empty()) {
            let heads: Vec<Option<usize>> = seg
                .iter()
                .map(|r| self.ctx.flat_width(&crate::builder::concat(r.head.clone())))
                .collect();
            let anchor_glued = seg
                .iter()
                .any(|r| !matches!(r.tail.first().map(|d| &d.0), Some(Inner::Space)));
            let max = heads.iter().flatten().copied().max().unwrap_or(0);
            // Only rows whose tail can be flat count for overflow; a tail that
            // breaks anyway (a nested multi-line expression) does not block
            // alignment.
            let overflow = seg.iter().zip(&heads).any(|(r, h)| match h {
                Some(_) => {
                    let tail = self
                        .ctx
                        .min_first_line(&crate::builder::concat(r.tail.clone()));
                    col + max + tail > width
                }
                None => true,
            });
            let aligned = !anchor_glued && !overflow && heads.iter().all(Option::is_some);
            for (r, h) in seg.iter().zip(&heads) {
                if !first_row {
                    self.newline(col);
                }
                first_row = false;
                self.run(
                    r.head
                        .iter()
                        .rev()
                        .map(|d| Cmd {
                            indent: col,
                            mode: Mode::Flat,
                            doc: d,
                        })
                        .collect(),
                );
                let mut tail: &[Doc] = &r.tail;
                if aligned {
                    let pad = max - h.unwrap_or(max);
                    let spaces = " ".repeat(pad + 1);
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

/// Width of leading indentation; a tab counts as one indent step.
fn visual_width(s: &str, step: usize) -> usize {
    s.chars().map(|c| if c == '\t' { step } else { 1 }).sum()
}

//! The only way to build a [`Doc`].
//!
//! `sep(i)`, `soft(i)` and `hard(i)` take the index of the token after the gap.
//! They emit nothing when that token is glued, so layout code cannot add a gap
//! between glued tokens. In debug builds they also fail an assertion, so the
//! mistake shows up in tests. Comments are emitted by the printer together
//! with the token they are attached to; layout code never emits them.

use crate::ir::{AlignFit, AlignItem, AlignRow, Doc, Inner};
use origami_syntax::{TokenIndex, TokenStream};

/// The original token `i`, with its attached comments.
pub fn token(i: TokenIndex) -> Doc {
    Doc(Inner::Token {
        index: i,
        blank: false,
    })
}

/// Token `i` at the start of a statement: one blank line before it (or before
/// its leading comments) is kept when the input had one.
pub fn token_keep_blank(i: TokenIndex) -> Doc {
    Doc(Inner::Token {
        index: i,
        blank: true,
    })
}

fn glued(tokens: &TokenStream, i: TokenIndex) -> bool {
    tokens.tokens.get(i.0 as usize).is_some_and(|t| t.glued)
}

fn gap(tokens: &TokenStream, i: TokenIndex, doc: Inner) -> Doc {
    let g = glued(tokens, i);
    debug_assert!(!g, "layout tried to add a gap before glued token {}", i.0);
    if g { empty() } else { Doc(doc) }
}

/// One space before token `i`, unless it is glued.
pub fn sep(tokens: &TokenStream, i: TokenIndex) -> Doc {
    gap(tokens, i, Inner::Space)
}

/// A space or (in a broken group) a line break before token `i`, unless it is
/// glued.
pub fn soft(tokens: &TokenStream, i: TokenIndex) -> Doc {
    gap(tokens, i, Inner::SoftLine)
}

/// Nothing or (in a broken group) a line break before token `i`; only valid
/// where the input has no gap requirement, so it is checked like `soft`.
pub fn soft_break(tokens: &TokenStream, i: TokenIndex) -> Doc {
    gap(tokens, i, Inner::SoftBreak)
}

/// `n` spaces when the enclosing group is broken, nothing when it is flat:
/// padding that only the broken layout has (ABAP-COND-001).
pub fn break_pad(n: usize) -> Doc {
    Doc(Inner::BreakPad(n as u32))
}

/// A line break before token `i`, unless it is glued.
pub fn hard(tokens: &TokenStream, i: TokenIndex) -> Doc {
    gap(tokens, i, Inner::HardLine)
}

/// The gap before token `i` as the input has it: nothing when glued, else one
/// space. For places where layout has no opinion.
pub fn keep(tokens: &TokenStream, i: TokenIndex) -> Doc {
    if glued(tokens, i) {
        empty()
    } else {
        Doc(Inner::Space)
    }
}

/// One space.
pub fn space() -> Doc {
    Doc(Inner::Space)
}

/// `n` spaces (padding for an aligned column).
pub fn spaces(n: usize) -> Doc {
    concat((0..n).map(|_| space()).collect())
}

pub fn empty() -> Doc {
    Doc(Inner::Concat(Vec::new()))
}

pub fn group(docs: Vec<Doc>) -> Doc {
    Doc(Inner::Group(docs))
}

pub fn indent(docs: Vec<Doc>) -> Doc {
    Doc(Inner::Indent(docs))
}

/// `n` indent levels, as one node.
pub fn indent_by(n: u32, docs: Vec<Doc>) -> Doc {
    Doc(Inner::IndentBy(n, docs))
}

/// `n` columns of indentation, whatever the indent width.
pub fn indent_cols(n: u32, docs: Vec<Doc>) -> Doc {
    Doc(Inner::IndentCols(n, docs))
}

/// `docs` start with a token whose leading comment lines are indented one
/// level deeper than the token: the closing bracket of a block.
pub fn lead_indent(docs: Vec<Doc>) -> Doc {
    Doc(Inner::LeadIndent(docs))
}

/// Continuation indent: two levels.
/// The continuation indent of ABAP-INDENT-001.
pub fn cont(docs: Vec<Doc>) -> Doc {
    Doc(Inner::Cont(docs))
}

pub fn hang(docs: Vec<Doc>) -> Doc {
    Doc(Inner::Hang(docs))
}

/// `(separator, content)` pairs; a separator (a soft line) breaks only when
/// its content does not fit on the current line.
pub fn fill(items: Vec<(Doc, Doc)>) -> Doc {
    Doc(Inner::Fill(items))
}

pub fn choice(first: Doc, second: Doc) -> Doc {
    Doc(Inner::Choice(Box::new(first), Box::new(second)))
}

pub fn align(items: Vec<AlignItem>) -> Doc {
    Doc(Inner::AlignGroup(items, AlignFit::FirstLine, false))
}

/// Aligned rows that fall back to single spaces when alignment would push a
/// row past the width ([`AlignFit::KeepLines`]); for rows that are whole
/// statements or chain items.
/// Like `align`, but always one row per line.
pub fn align_broken(items: Vec<AlignItem>) -> Doc {
    Doc(Inner::AlignGroup(items, AlignFit::FirstLine, true))
}

pub fn align_lines(items: Vec<AlignItem>) -> Doc {
    Doc(Inner::AlignGroup(items, AlignFit::KeepLines, false))
}

pub fn row(head: Vec<Doc>, tail: Vec<Doc>) -> AlignItem {
    AlignItem::Row(AlignRow {
        lead: Vec::new(),
        head,
        tail,
        anchor: 0,
    })
}

/// A row whose anchor (`anchor` characters wide) is aligned at its right end.
pub fn row_right(head: Vec<Doc>, anchor: usize, tail: Vec<Doc>) -> AlignItem {
    AlignItem::Row(AlignRow {
        lead: Vec::new(),
        head,
        tail,
        anchor,
    })
}

/// A row with leading columns. Each non-empty cell with content after it
/// gets a trailing space; an empty cell is padding only (see `AlignRow`).
pub fn row_cols(lead: Vec<Vec<Doc>>, head: Vec<Doc>, tail: Vec<Doc>) -> AlignItem {
    let n = lead.len();
    let more: Vec<bool> = (0..n)
        .map(|c| {
            lead[c + 1..].iter().any(|x| !x.is_empty()) || !head.is_empty() || !tail.is_empty()
        })
        .collect();
    let lead = lead
        .into_iter()
        .zip(more)
        .map(|(mut c, more)| {
            if !c.is_empty() && more {
                c.push(Doc(Inner::Space));
            }
            c
        })
        .collect();
    AlignItem::Row(AlignRow {
        lead,
        head,
        tail,
        anchor: 0,
    })
}

/// An unaligned line inside an alignment group.
pub fn line(docs: Vec<Doc>) -> AlignItem {
    AlignItem::Line(docs)
}

pub fn concat(docs: Vec<Doc>) -> Doc {
    Doc(Inner::Concat(docs))
}

/// `doc`, cheap to clone: for a doc used in both branches of a [`choice`].
pub fn shared(doc: Doc) -> Doc {
    Doc(Inner::Shared(std::sync::Arc::new(doc)))
}

/// `blank`: keep one blank line before the unit when the input had one.
pub fn unknown(first: TokenIndex, last: TokenIndex, blank: bool) -> Doc {
    Doc(Inner::Unknown { first, last, blank })
}

/// `blank`: keep one blank line before the unit when the input had one.
pub fn verbatim(first: TokenIndex, last: TokenIndex, blank: bool) -> Doc {
    Doc(Inner::Verbatim { first, last, blank })
}

/// `blank`: keep one blank line before the unit when the input had one.
pub fn opaque(first: TokenIndex, last: TokenIndex, blank: bool) -> Doc {
    Doc(Inner::Opaque { first, last, blank })
}

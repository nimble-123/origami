//! The only way to build a [`Doc`].
//!
//! `sep(i)`, `soft(i)` and `hard(i)` take the index of the token after the gap.
//! They emit nothing when that token is glued, so layout code cannot add a gap
//! between glued tokens. In debug builds they also fail an assertion, so the
//! mistake shows up in tests. Comments are emitted by the printer together
//! with the token they are attached to; layout code never emits them.

use crate::ir::{AlignItem, AlignRow, Doc, Inner};
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

pub fn empty() -> Doc {
    Doc(Inner::Concat(Vec::new()))
}

pub fn group(docs: Vec<Doc>) -> Doc {
    Doc(Inner::Group(docs))
}

pub fn indent(docs: Vec<Doc>) -> Doc {
    Doc(Inner::Indent(docs))
}

/// Continuation indent: two levels.
pub fn cont(docs: Vec<Doc>) -> Doc {
    indent(vec![indent(docs)])
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
    Doc(Inner::AlignGroup(items))
}

pub fn row(head: Vec<Doc>, tail: Vec<Doc>) -> AlignItem {
    AlignItem::Row(AlignRow { head, tail })
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

//! The only way to build a [`Doc`].
//!
//! `sep(i)`, `soft(i)` and `hard(i)` take the index of the token after the gap.
//! They emit nothing when that token is glued, so layout code cannot add a gap
//! between glued tokens. In debug builds they also fail an assertion, so the
//! mistake shows up in tests.

use crate::ir::{AlignItem, Doc, Inner};
use origami_syntax::{TokenIndex, TokenStream};
use std::ops::Range;

/// The original token `i`. The printer emits its attached comments with it.
pub fn token(i: TokenIndex) -> Doc {
    Doc(Inner::Token(i))
}

fn gap(tokens: &TokenStream, i: TokenIndex, doc: Inner) -> Doc {
    let glued = tokens.tokens.get(i.0 as usize).is_some_and(|t| t.glued);
    debug_assert!(
        !glued,
        "layout tried to add a gap before glued token {}",
        i.0
    );
    if glued {
        Doc(Inner::Concat(Vec::new()))
    } else {
        Doc(doc)
    }
}

/// One space before token `i`, unless it is glued.
pub fn sep(tokens: &TokenStream, i: TokenIndex) -> Doc {
    gap(tokens, i, Inner::Space)
}

/// A soft line break before token `i`, unless it is glued.
pub fn soft(tokens: &TokenStream, i: TokenIndex) -> Doc {
    gap(tokens, i, Inner::SoftLine)
}

/// A hard line break before token `i`, unless it is glued.
pub fn hard(tokens: &TokenStream, i: TokenIndex) -> Doc {
    gap(tokens, i, Inner::HardLine)
}

pub fn blank_line() -> Doc {
    Doc(Inner::BlankLine)
}

pub fn group(docs: Vec<Doc>) -> Doc {
    Doc(Inner::Group(docs))
}

pub fn indent(docs: Vec<Doc>) -> Doc {
    Doc(Inner::Indent(docs))
}

pub fn align(items: Vec<AlignItem>) -> Doc {
    Doc(Inner::AlignGroup(items))
}

pub fn concat(docs: Vec<Doc>) -> Doc {
    Doc(Inner::Concat(docs))
}

pub fn unknown(range: Range<u32>) -> Doc {
    Doc(Inner::Unknown(range))
}

pub fn verbatim(range: Range<u32>) -> Doc {
    Doc(Inner::Verbatim(range))
}

pub fn opaque(range: Range<u32>) -> Doc {
    Doc(Inner::Opaque(range))
}

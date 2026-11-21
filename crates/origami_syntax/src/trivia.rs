//! Whitespace and comment model.
//!
//! A comment records its syntax (which comment form it is) and its placement:
//! `EndOfLine` when a token precedes it on the same line, `OwnLine` otherwise.
//! [`attach`] anchors every comment to a token: end-of-line comments trail the
//! preceding token, own-line comments lead the next token, or the virtual
//! end-of-file anchor when no token follows.

use crate::token::{Token, TokenIndex};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentSyntax {
    /// ABAP `*` comment in column 1.
    AbapStar,
    /// ABAP `"` comment.
    AbapQuote,
    /// ABAP Doc `"!`.
    AbapDoc,
    /// ABAP pseudo-comment `"#EC`.
    AbapPseudo,
    /// CDS `//` or `--` comment.
    CdsLine,
    /// CDS `/* */` comment.
    CdsBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    OwnLine,
    EndOfLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriviaKind {
    Whitespace,
    Newline,
    Comment {
        syntax: CommentSyntax,
        placement: Placement,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriviaPosition {
    Leading,
    Trailing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub start: u32,
    pub end: u32,
    pub anchor: TokenIndex,
    pub position: TriviaPosition,
}

impl Trivia {
    pub fn new(kind: TriviaKind, span: Range<usize>) -> Trivia {
        Trivia {
            kind,
            start: span.start as u32,
            end: span.end as u32,
            anchor: TokenIndex(0),
            position: TriviaPosition::Leading,
        }
    }

    pub fn is_comment(&self) -> bool {
        matches!(self.kind, TriviaKind::Comment { .. })
    }

    pub fn text<'a>(&self, src: &'a str) -> &'a str {
        &src[self.start as usize..self.end as usize]
    }
}

/// Sets `anchor` and `position` of every trivia piece. Both slices must be in
/// source order.
pub fn attach(tokens: &[Token], trivia: &mut [Trivia]) {
    let mut next = 0usize; // index of the first token starting after the trivia
    for t in trivia.iter_mut() {
        while next < tokens.len() && tokens[next].start < t.start {
            next += 1;
        }
        let trailing = matches!(
            t.kind,
            TriviaKind::Comment {
                placement: Placement::EndOfLine,
                ..
            }
        );
        if trailing && next > 0 {
            t.anchor = TokenIndex(next as u32 - 1);
            t.position = TriviaPosition::Trailing;
        } else {
            t.anchor = TokenIndex(next as u32);
            t.position = TriviaPosition::Leading;
        }
    }
}

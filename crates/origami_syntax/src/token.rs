//! Token model shared by all languages.

use crate::trivia::Trivia;
use std::ops::Range;

/// Index of a token in a [`TokenStream`]. `TokenIndex(tokens.len())` is the
/// virtual end-of-file anchor for trailing comments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenIndex(pub u32);

/// A token. `kind` values are defined by each language crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: u16,
    pub start: u32,
    pub end: u32,
    /// True when no whitespace, line break or comment separates this token from
    /// the previous token. The output keeps it, except that a token marked
    /// `may_glue` may become glued.
    pub glued: bool,
    /// Set only by a parser, for tokens consumed as keywords of a recognized
    /// statement. These tokens may be re-cased.
    pub keyword: bool,
    /// Set only by a parser, for identifier tokens of a recognized statement
    /// (ABAP). The printer writes them in lower case when they contain no
    /// lower-case letter, so they may be re-cased too.
    pub ident: bool,
    /// Set only by the ABAP crate, for a statement's closing period and a
    /// comma outside brackets that is not glued (ABAP-SPACE-001). The output
    /// may glue it to the previous token; the comments between the two then
    /// move behind it (see `compare::equivalent`).
    pub may_glue: bool,
}

impl Token {
    pub fn span(&self) -> Range<usize> {
        self.start as usize..self.end as usize
    }

    pub fn text<'a>(&self, src: &'a str) -> &'a str {
        &src[self.span()]
    }
}

/// All tokens and all trivia of one source, both in source order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TokenStream {
    pub tokens: Vec<Token>,
    pub trivia: Vec<Trivia>,
}

impl TokenStream {
    pub fn eof_anchor(&self) -> TokenIndex {
        TokenIndex(self.tokens.len() as u32)
    }

    /// Index of the token whose span starts at `offset`, if any.
    pub fn index_at(&self, offset: u32) -> Option<TokenIndex> {
        self.tokens
            .binary_search_by_key(&offset, |t| t.start)
            .ok()
            .map(|i| TokenIndex(i as u32))
    }
}

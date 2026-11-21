//! Doc IR. The variants are private to this crate; layout code builds docs
//! through [`crate::builder`].

use origami_syntax::TokenIndex;
use std::ops::Range;

/// A layout document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doc(pub(crate) Inner);

// The M0 printer only implements identity mode; the full printer that reads
// these variants arrives in M1.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Inner {
    /// The original token, with its attached comments.
    Token(TokenIndex),
    /// One space.
    Space,
    /// A space if the enclosing group fits, a line break otherwise.
    SoftLine,
    /// Always a line break.
    HardLine,
    /// An empty line; consecutive ones collapse to one.
    BlankLine,
    /// Flat if it fits, broken otherwise.
    Group(Vec<Doc>),
    /// Adds one indent level to breaks inside it.
    Indent(Vec<Doc>),
    /// Aligned rows (D7).
    AlignGroup(Vec<AlignItem>),
    /// Unknown statement: re-indent and shrink runs of spaces.
    Unknown(Range<u32>),
    /// Quarantined unit: re-indent only.
    Verbatim(Range<u32>),
    /// Byte-exact, never re-indented.
    Opaque(Range<u32>),
    Concat(Vec<Doc>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlignItem {
    Row(AlignRow),
    /// A comment line, by index into `TokenStream::trivia`. Ends a segment.
    Comment(u32),
    /// A blank line. Ends a segment.
    Blank,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignRow {
    pub head: Vec<Doc>,
    pub anchor: TokenIndex,
    pub tail: Vec<Doc>,
}

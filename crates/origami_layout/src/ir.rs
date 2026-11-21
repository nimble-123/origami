//! Doc IR. The variants are private to this crate; layout code builds docs
//! through [`crate::builder`].

use origami_syntax::TokenIndex;

/// A layout document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doc(pub(crate) Inner);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Inner {
    /// The original token, with its attached comments. `blank` allows one
    /// blank line before it (and before its leading comments) when the input
    /// had one.
    Token {
        index: TokenIndex,
        blank: bool,
    },
    /// One space.
    Space,
    /// A space if the enclosing group fits, a line break otherwise.
    SoftLine,
    /// A line break if the enclosing group is broken, nothing otherwise.
    SoftBreak,
    /// Always a line break.
    HardLine,
    /// Flat if it fits, broken otherwise.
    Group(Vec<Doc>),
    /// Adds one indent level to breaks inside it.
    Indent(Vec<Doc>),
    /// Breaks inside it continue at the current column (hanging indent).
    Hang(Vec<Doc>),
    /// `first` if every line of it fits into the width, else `second`.
    Choice(Box<Doc>, Box<Doc>),
    /// Aligned rows (design decision D7); printed one row per line.
    AlignGroup(Vec<AlignItem>),
    /// Unknown statement: re-indent and shrink runs of spaces to one.
    Unknown {
        first: TokenIndex,
        last: TokenIndex,
        blank: bool,
    },
    /// Quarantined unit: re-indent only.
    Verbatim {
        first: TokenIndex,
        last: TokenIndex,
        blank: bool,
    },
    /// Byte-exact, never re-indented.
    Opaque {
        first: TokenIndex,
        last: TokenIndex,
        blank: bool,
    },
    Concat(Vec<Doc>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlignItem {
    Row(AlignRow),
    /// Ends the current alignment segment (a blank line or comment line in
    /// the input).
    Break,
}

/// One row: `head` is padded to the widest head of its segment, then `tail`
/// follows. `tail` starts with the anchor token (for example `=` or `TYPE`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignRow {
    pub head: Vec<Doc>,
    pub tail: Vec<Doc>,
}

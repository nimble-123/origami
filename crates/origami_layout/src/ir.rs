//! Doc IR. The variants are private to this crate; layout code builds docs
//! through [`crate::builder`].

use origami_syntax::TokenIndex;
use std::sync::Arc;

/// A layout document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doc(pub(crate) Inner);

impl Doc {
    /// For a concatenation: its first element and the rest.
    pub fn into_parts(self) -> Option<(Doc, Doc)> {
        match self.0 {
            Inner::Concat(mut v) if !v.is_empty() => {
                let first = v.remove(0);
                Some((first, Doc(Inner::Concat(v))))
            }
            _ => None,
        }
    }
}

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
    /// Adds the continuation indent (`PrintOptions::cont_levels` levels) to
    /// breaks inside it (ABAP-INDENT-001).
    Cont(Vec<Doc>),
    /// Breaks inside it continue at the current column (hanging indent).
    Hang(Vec<Doc>),
    /// Clauses: each separator breaks only when the content after it does not
    /// fit on the current line.
    Fill(Vec<(Doc, Doc)>),
    /// `first` if every line of it fits into the width, else `second`.
    Choice(Box<Doc>, Box<Doc>),
    /// Aligned rows (design decision D7); printed one row per line.
    AlignGroup(Vec<AlignItem>, AlignFit),
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
    /// A doc that appears in several places (both branches of a `Choice`)
    /// without being copied: nested choices would otherwise double the doc
    /// at every level.
    Shared(Arc<Doc>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlignItem {
    Row(AlignRow),
    /// Ends the current alignment segment (a blank line or comment line in
    /// the input).
    Break,
    /// A line of its own that is not aligned and does not end the segment
    /// (a chain item without the anchor).
    Line(Vec<Doc>),
}

impl AlignItem {
    /// The docs of a row (head, then tail) or line, in order; `None` for a
    /// segment break.
    pub(crate) fn docs(&self) -> Option<impl DoubleEndedIterator<Item = &Doc>> {
        match self {
            AlignItem::Row(r) => Some(r.head.iter().chain(r.tail.iter())),
            AlignItem::Line(d) => Some(d.iter().chain([].iter())),
            AlignItem::Break => None,
        }
    }
}

/// One row: `head` is padded to the widest head of its segment, then `tail`
/// follows. `tail` starts with the gap before the anchor token (for example
/// `=` or `TYPE`). With `anchor > 0` (the anchor's width), anchors of
/// different widths are right-aligned in a column as wide as the widest one
/// (` =` above `-=`), as ABAP Cleaner aligns assignment operators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignRow {
    pub head: Vec<Doc>,
    pub tail: Vec<Doc>,
    pub anchor: usize,
}

/// When a segment is too wide to align, it falls back to single spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignFit {
    /// Too wide: a row's first line (up to its first possible break) would
    /// end past the width.
    FirstLine,
    /// Too wide: alignment would push a row that fits on one line past the
    /// width, or the first line of a longer row that fits past it. Rows that
    /// are too wide anyway do not count.
    KeepLines,
}

//! Types for line-break explanations, shared by the core and the language
//! crates. Language crates record [`RuleSpan`]s while they lay out a source;
//! the core turns the line breaks origami introduced into [`Note`]s.

use crate::token::TokenIndex;
use std::ops::RangeInclusive;

/// A STYLE rule ID such as `"ABAP-CALL-001"`.
pub type RuleId = &'static str;

/// Tokens `first..=last` form one construct laid out by `rule`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleSpan {
    pub first: TokenIndex,
    pub last: TokenIndex,
    pub rule: RuleId,
}

impl RuleSpan {
    /// Whether a line break before token `k` lies inside this construct.
    pub fn contains_break(&self, k: TokenIndex) -> bool {
        self.first < k && k <= self.last
    }

    pub fn len(&self) -> u32 {
        self.last.0 - self.first.0 + 1
    }

    pub fn is_empty(&self) -> bool {
        false
    }
}

/// What a width column measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The end of the construct.
    Construct,
    /// The end of the line that holds the end of the construct.
    Line,
}

/// Why origami introduced a line break.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakCause {
    /// The construct (or its line) would end past the limit on one line.
    Width {
        column: u32,
        limit: u16,
        scope: Scope,
    },
    /// A style rule always breaks here.
    Rule,
    /// The token before the break carries an end-of-line comment.
    Comment { text: String },
    /// Broken for width, but no measured column exceeds the limit (a choice
    /// between layout variants).
    Layout,
}

/// One explanation: the introduced breaks of one construct with one cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// 1-based lines of the construct in the formatted output.
    pub lines: RangeInclusive<u32>,
    /// First tokens of the lines this note explains.
    pub break_tokens: Vec<TokenIndex>,
    pub rule: Option<RuleId>,
    pub cause: BreakCause,
    /// The finished sentence.
    pub text: String,
}

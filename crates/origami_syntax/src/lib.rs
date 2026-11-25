//! Shared syntax primitives for origami: decoded source text, tokens, comments,
//! diagnostics, types shared between the core and the language crates, and the
//! token-stream comparison used by the self-check.
//!
//! This crate contains no knowledge of any SAP language.

pub mod compare;
pub mod diagnostic;
pub mod explain;
pub mod lang;
pub mod source;
pub mod token;
pub mod trivia;

pub use compare::{Mismatch, MismatchKind, equivalent, spaced_comment};
pub use diagnostic::{Code, Diagnostic, Severity};
pub use explain::{BreakCause, Note, RuleId, RuleSpan, Scope};
pub use lang::{
    Case, Edition, LangOptions, LangResult, Language, MAX_NESTING, Stats, Style, Unit, UnitId,
    max_output_len, output_limit_diagnostic,
};
pub use source::{LineEnding, LineIndex, Position, SourceText};
pub use token::{Token, TokenIndex, TokenStream};
pub use trivia::{CommentSyntax, Placement, Trivia, TriviaKind, TriviaPosition};

//! Token-stream equality for the self-check (see `docs/INVARIANTS.md`).
//!
//! Two streams are equivalent when they have the same tokens (kind and text;
//! case-insensitive only for tokens the input marks as keywords), the same
//! `glued` flags, and the same comments: same text, same syntax, same
//! placement, attached to the same token in the same position.

use crate::token::{TokenIndex, TokenStream};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismatchKind {
    Text,
    Kind,
    Glued,
    CommentText,
    CommentAnchor,
    CommentKind,
    Count,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mismatch {
    /// Input token the mismatch belongs to. For `Count`, the first position
    /// where the two streams differ.
    pub input_index: TokenIndex,
    pub kind: MismatchKind,
}

/// Compares the input stream with the re-lexed output stream. Keyword flags
/// are taken from `input`; output tokens always have `keyword == false`.
pub fn equivalent(
    input: &TokenStream,
    input_src: &str,
    output: &TokenStream,
    output_src: &str,
) -> Result<(), Vec<Mismatch>> {
    let mut mismatches = Vec::new();
    let n = input.tokens.len().min(output.tokens.len());
    // Different counts shift every later token, so only the first difference
    // is meaningful.
    if input.tokens.len() != output.tokens.len() {
        let first = (0..n)
            .find(|&i| input.tokens[i].text(input_src) != output.tokens[i].text(output_src))
            .unwrap_or(n);
        return Err(vec![Mismatch {
            input_index: TokenIndex(first.min(input.tokens.len().saturating_sub(1)) as u32),
            kind: MismatchKind::Count,
        }]);
    }
    for i in 0..n {
        let a = &input.tokens[i];
        let b = &output.tokens[i];
        let at = TokenIndex(i as u32);
        let (ta, tb) = (a.text(input_src), b.text(output_src));
        if a.kind != b.kind {
            mismatches.push(Mismatch {
                input_index: at,
                kind: MismatchKind::Kind,
            });
        } else if !(ta == tb || (a.keyword && ta.eq_ignore_ascii_case(tb))) {
            mismatches.push(Mismatch {
                input_index: at,
                kind: MismatchKind::Text,
            });
        } else if i > 0 && a.glued != b.glued {
            mismatches.push(Mismatch {
                input_index: at,
                kind: MismatchKind::Glued,
            });
        }
    }
    let comments_a: Vec<_> = input.trivia.iter().filter(|t| t.is_comment()).collect();
    let comments_b: Vec<_> = output.trivia.iter().filter(|t| t.is_comment()).collect();
    for (a, b) in comments_a.iter().zip(comments_b.iter()) {
        let at = clamp(a.anchor, input);
        if a.text(input_src) != b.text(output_src) {
            mismatches.push(Mismatch {
                input_index: at,
                kind: MismatchKind::CommentText,
            });
        } else if a.kind != b.kind {
            mismatches.push(Mismatch {
                input_index: at,
                kind: MismatchKind::CommentKind,
            });
        } else if a.anchor != b.anchor || a.position != b.position {
            mismatches.push(Mismatch {
                input_index: at,
                kind: MismatchKind::CommentAnchor,
            });
        }
    }
    if comments_a.len() != comments_b.len() {
        let at = comments_a
            .get(comments_b.len())
            .or(comments_a.last())
            .map(|c| clamp(c.anchor, input))
            .unwrap_or(TokenIndex(0));
        mismatches.push(Mismatch {
            input_index: at,
            kind: MismatchKind::CommentText,
        });
    }
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(mismatches)
    }
}

fn clamp(index: TokenIndex, stream: &TokenStream) -> TokenIndex {
    TokenIndex(index.0.min(stream.tokens.len().saturating_sub(1) as u32))
}

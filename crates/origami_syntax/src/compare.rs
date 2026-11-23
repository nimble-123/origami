//! Token-stream equality for the self-check (see `docs/INVARIANTS.md`).
//!
//! Two streams are equivalent when they have the same tokens (kind and text;
//! case-insensitive only for tokens the input marks as keywords or
//! identifiers), the same
//! `glued` flags, and the same comments: same text, same syntax, same
//! placement, attached to the same token in the same position.
//!
//! Two exceptions, both as narrow as their rules:
//!
//! * ABAP-SPACE-001: a token the input marks `may_glue` (a period or comma)
//!   may become glued. When it does, the comments between it and the
//!   previous token move behind it: an end-of-line comment that trailed the
//!   previous token trails the glued token, and a comment line that led the
//!   glued token leads the next one.
//! * ABAP-DOC-001: an ABAP Doc comment `"! @parameter name | …` (also
//!   `@raising`, `@exception`) may change the blanks after the tag and
//!   between the name and `|`; nothing else (see [`abap_doc_aligned`]).

use crate::token::{TokenIndex, TokenStream};
use crate::trivia::{CommentSyntax, Trivia, TriviaKind, TriviaPosition};

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
    // Same text, or a keyword that only changed case.
    let same_text = |i: usize| {
        let a = &input.tokens[i];
        let (ta, tb) = (a.text(input_src), output.tokens[i].text(output_src));
        ta == tb || ((a.keyword || a.ident) && ta.eq_ignore_ascii_case(tb))
    };
    // Different counts shift every later token, so only the first difference
    // is meaningful. A keyword that only changed case is not a difference:
    // the mismatch must point at the unit that lost or gained tokens.
    if input.tokens.len() != output.tokens.len() {
        let first = (0..n)
            .find(|&i| input.tokens[i].kind != output.tokens[i].kind || !same_text(i))
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
        if a.kind != b.kind {
            mismatches.push(Mismatch {
                input_index: at,
                kind: MismatchKind::Kind,
            });
        } else if !same_text(i) {
            mismatches.push(Mismatch {
                input_index: at,
                kind: MismatchKind::Text,
            });
        } else if i > 0 && a.glued != b.glued && !(a.may_glue && b.glued) {
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
        let (ta, tb) = (a.text(input_src), b.text(output_src));
        if ta != tb && !(a.kind == b.kind && is_abap_doc(a) && abap_doc_aligned(ta, tb)) {
            mismatches.push(Mismatch {
                input_index: at,
                kind: MismatchKind::CommentText,
            });
        } else if a.kind != b.kind {
            mismatches.push(Mismatch {
                input_index: at,
                kind: MismatchKind::CommentKind,
            });
        } else if (a.anchor != b.anchor || a.position != b.position) && !moved(input, output, a, b)
        {
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

/// ABAP-SPACE-001: comment `a` stood between a `may_glue` token and the
/// token before it, and in the output (`b`) that token is glued and the
/// comment is one token later, in the same position.
fn moved(input: &TokenStream, output: &TokenStream, a: &Trivia, b: &Trivia) -> bool {
    let glue = match a.position {
        // Trailed the previous token, now trails the glued one.
        TriviaPosition::Trailing => a.anchor.0 + 1,
        // Led the glued token, now leads the next one.
        TriviaPosition::Leading => a.anchor.0,
    } as usize;
    let marked = input
        .tokens
        .get(glue)
        .is_some_and(|t| t.may_glue && !t.glued);
    let glued = output.tokens.get(glue).is_some_and(|t| t.glued);
    marked && glued && b.position == a.position && b.anchor.0 == a.anchor.0 + 1
}

fn is_abap_doc(t: &Trivia) -> bool {
    matches!(
        t.kind,
        TriviaKind::Comment {
            syntax: CommentSyntax::AbapDoc,
            ..
        }
    )
}

/// ABAP-DOC-001: `a` and `b` are the same ABAP Doc line
/// `"! @parameter name | text` (or `@raising`, `@exception`, in any letter
/// case), except for the blanks after the tag and between the name and `|`.
pub fn abap_doc_aligned(a: &str, b: &str) -> bool {
    match (abap_doc_parts(a), abap_doc_parts(b)) {
        (Some(pa), Some(pb)) => pa == pb,
        _ => false,
    }
}

/// `(up to the end of the tag, name, from the bar)` of an ABAP Doc tag line
/// `"! @parameter name | text` (also `@raising`, `@exception`, any letter
/// case): at least one blank after the tag, the name ends at a blank or `|`,
/// and only blanks stand between the name and `|`.
pub fn abap_doc_parts(s: &str) -> Option<(&str, &str, &str)> {
    let blank = |c: char| c == ' ' || c == '\t';
    let rest = s.strip_prefix("\"!")?.trim_start_matches(blank);
    let tag_len = ["@parameter", "@raising", "@exception"]
        .iter()
        .find(|t| {
            rest.get(..t.len())
                .is_some_and(|r| r.eq_ignore_ascii_case(t))
        })?
        .len();
    let tag_end = s.len() - rest.len() + tag_len;
    let after = &s[tag_end..];
    let name_part = after.trim_start_matches(blank);
    if name_part.len() == after.len() {
        return None; // no blank after the tag
    }
    let name_len = name_part.find(|c: char| blank(c) || c == '|')?;
    let bar = name_part[name_len..].trim_start_matches(blank);
    if name_len == 0 || !bar.starts_with('|') {
        return None;
    }
    Some((&s[..tag_end], &name_part[..name_len], bar))
}

fn clamp(index: TokenIndex, stream: &TokenStream) -> TokenIndex {
    TokenIndex(index.0.min(stream.tokens.len().saturating_sub(1) as u32))
}

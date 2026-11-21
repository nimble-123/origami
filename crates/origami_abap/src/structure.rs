//! Structural pass: statements, brackets and block pairs.
//!
//! Together with the lexer (E001) and decoding (E005), this pass is the only
//! source of structural errors: E002 unbalanced bracket, E003 missing period,
//! E004 block-pair mismatch.

use crate::block_pairs::{Role, classify};
use crate::kinds::*;
use origami_syntax::{Code, Diagnostic, TokenStream};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// Token range, including the terminating period.
    pub tokens: Range<usize>,
    /// Block nesting depth.
    pub depth: u32,
}

pub fn check(text: &str, stream: &TokenStream) -> Result<Vec<Statement>, Diagnostic> {
    let tokens = &stream.tokens;
    let mut statements = Vec::new();
    let mut blocks: Vec<(&'static str, u32)> = Vec::new(); // (closer, opener offset)
    let mut brackets: Vec<(u16, u32)> = Vec::new(); // (closing kind, offset)
    let mut start = 0usize;

    let err = |code: Code, msg: String, at: u32| Diagnostic::new(code, msg, at..at + 1);

    for (k, t) in tokens.iter().enumerate() {
        match t.kind {
            LPAREN => brackets.push((RPAREN, t.start)),
            LBRACKET => brackets.push((RBRACKET, t.start)),
            LBRACE => brackets.push((RBRACE, t.start)),
            RPAREN | RBRACKET | RBRACE => match brackets.pop() {
                Some((close, _)) if close == t.kind => {}
                _ => {
                    return Err(err(
                        Code::E002,
                        "Closing bracket without opening bracket".into(),
                        t.start,
                    ));
                }
            },
            PERIOD if brackets.is_empty() => {
                let range = start..k + 1;
                let mut words = Vec::new();
                let mut depth0 = Vec::new();
                let mut depth = 0i32;
                for tok in &tokens[range.clone()] {
                    match tok.kind {
                        LPAREN | LBRACKET => depth += 1,
                        RPAREN | RBRACKET => depth -= 1,
                        WORD => {
                            let w = tok.text(text).to_ascii_uppercase();
                            if depth == 0 {
                                depth0.push(w.clone());
                            }
                            words.push(w);
                        }
                        _ => {}
                    }
                }
                let opener_at = tokens[start].start;
                let level = blocks.len() as u32;
                let depth_here = match classify(&words, &depth0) {
                    Some(Role::Opener(closer)) => {
                        blocks.push((closer, opener_at));
                        level
                    }
                    Some(Role::Closer) => match blocks.pop() {
                        Some((closer, _)) if closer == words[0] => level - 1,
                        Some((closer, _)) => {
                            return Err(err(
                                Code::E004,
                                format!("Found {} but expected {closer}", words[0]),
                                opener_at,
                            ));
                        }
                        None => {
                            return Err(err(
                                Code::E004,
                                format!("{} without a matching opening statement", words[0]),
                                opener_at,
                            ));
                        }
                    },
                    Some(Role::Intermediate) => level.saturating_sub(1),
                    None => level,
                };
                statements.push(Statement {
                    tokens: range,
                    depth: depth_here,
                });
                start = k + 1;
            }
            PERIOD => {
                let (_, at) = brackets[brackets.len() - 1];
                return Err(err(
                    Code::E002,
                    "Bracket not closed before the period".into(),
                    at,
                ));
            }
            _ => {}
        }
    }
    if let Some((_, at)) = brackets.last() {
        return Err(err(Code::E002, "Bracket not closed".into(), *at));
    }
    if start < tokens.len() {
        let at = tokens[tokens.len() - 1].start;
        return Err(err(
            Code::E003,
            "Statement is not terminated by a period".into(),
            at,
        ));
    }
    if let Some((closer, at)) = blocks.last() {
        return Err(err(
            Code::E004,
            format!("Missing {closer} for this block"),
            *at,
        ));
    }
    Ok(statements)
}

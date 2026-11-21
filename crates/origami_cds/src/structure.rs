//! Structural pass for the CDS family: bracket balance (E002) and units.
//!
//! The CDS family raises no E003: a missing `;` is not a structural error, so
//! the parser's fallback handles it.

use crate::kinds::*;
use origami_syntax::{Code, Diagnostic, TokenIndex, TokenStream, Unit};

pub fn check(stream: &TokenStream) -> Result<(), Diagnostic> {
    let mut stack: Vec<(u16, u32)> = Vec::new();
    for t in &stream.tokens {
        let close = match t.kind {
            LBRACE => Some(RBRACE),
            LPAREN => Some(RPAREN),
            LBRACKET => Some(RBRACKET),
            _ => None,
        };
        if let Some(c) = close {
            stack.push((c, t.start));
        } else if matches!(t.kind, RBRACE | RPAREN | RBRACKET) {
            match stack.pop() {
                Some((c, _)) if c == t.kind => {}
                _ => {
                    return Err(Diagnostic::new(
                        Code::E002,
                        "Closing bracket without opening bracket",
                        t.start..t.start + 1,
                    ));
                }
            }
        }
    }
    if let Some((_, at)) = stack.last() {
        return Err(Diagnostic::new(
            Code::E002,
            "Bracket not closed",
            *at..*at + 1,
        ));
    }
    Ok(())
}

/// Splits the token stream into units. M0 approximation: a unit ends after a
/// `;`, a `,`, a `{` or a `}`. M2 refines this to annotations, list elements,
/// statements and headers.
pub fn units(stream: &TokenStream) -> Vec<Unit> {
    let mut units = Vec::new();
    let mut start = 0usize;
    for (k, t) in stream.tokens.iter().enumerate() {
        if matches!(t.kind, SEMICOLON | COMMA | LBRACE | RBRACE) {
            units.push(Unit {
                id: units.len() as u32,
                tokens: TokenIndex(start as u32)..TokenIndex(k as u32 + 1),
            });
            start = k + 1;
        }
    }
    if start < stream.tokens.len() {
        units.push(Unit {
            id: units.len() as u32,
            tokens: TokenIndex(start as u32)..TokenIndex(stream.tokens.len() as u32),
        });
    }
    units
}

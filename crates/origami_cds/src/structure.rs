//! Structural pass for the CDS family: bracket balance (E002) and units.
//!
//! The CDS family raises no E003: a missing `;` is not a structural error, so
//! the parser's fallback handles it.

use crate::kinds::*;
use origami_syntax::{Code, Diagnostic, TokenStream};

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

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
    /// Indent level (STYLE ABAP-BLOCK-001, ABAP-CLASS-001): one per block;
    /// `CASE`, `TRY` and class definitions add one more for their bodies, so
    /// `WHEN`, `CATCH` and the sections sit one level in.
    pub depth: u32,
    /// Block role of the statement, if any.
    pub role: Option<Role>,
}

/// False when the statement's first word is used as a name: followed by a
/// glued `(`, `[` or selector, or by an assignment operator.
fn keyword_position(
    text: &str,
    tokens: &[origami_syntax::Token],
    start: usize,
    end: usize,
) -> bool {
    let (Some(first), Some(next)) = (tokens.get(start), tokens.get(start + 1)) else {
        return true;
    };
    if start + 1 >= end || first.kind != WORD {
        return true;
    }
    if next.glued && matches!(next.kind, LPAREN | LBRACKET | SELECTOR) {
        return false;
    }
    let op = next.text(text);
    !(next.kind == WORD && matches!(op, "=" | "?=" | "+=" | "-=" | "*=" | "/=" | "&&="))
}

/// Closer of the implicit block an event keyword opens (an event block has
/// no closing statement; it ends at the next event or processing block).
const EVENT: &str = "";

/// Event keywords of reports and module pools (ABAP-EVENT-001).
fn is_event(words: &[String]) -> bool {
    let w = |k: usize| words.get(k).map(String::as_str).unwrap_or("");
    match w(0) {
        "LOAD-OF-PROGRAM" | "INITIALIZATION" | "START-OF-SELECTION" | "END-OF-SELECTION"
        | "TOP-OF-PAGE" | "END-OF-PAGE" => true,
        // Dynpro flow logic: PROCESS BEFORE OUTPUT / AFTER INPUT /
        // ON HELP-REQUEST / ON VALUE-REQUEST.
        "PROCESS" => matches!(w(1), "BEFORE" | "AFTER" | "ON"),
        "AT" => {
            matches!(w(1), "SELECTION-SCREEN" | "LINE-SELECTION" | "USER-COMMAND")
                || (w(1).starts_with("PF") && w(1)[2..].parse::<u8>().is_ok())
        }
        // Logical database event `GET node [LATE] [FIELDS …]`: any `GET`
        // that is not one of the statements below.
        "GET" => {
            !w(1).is_empty()
                && !matches!(
                    w(1),
                    "BADI"
                        | "BIT"
                        | "CURSOR"
                        | "DATASET"
                        | "LOCALE"
                        | "PARAMETER"
                        | "PF-STATUS"
                        | "PROPERTY"
                        | "REFERENCE"
                        | "RUN"
                        | "TIME"
                )
        }
        _ => false,
    }
}

/// Closer of a structure that opens in one declaration and closes in a later
/// one (`DATA: BEGIN OF s.` … `DATA: END OF s.`); its body is indented.
const END_OF: &str = "END OF";

/// Net number of structures a declaration opens (positive) or closes.
fn struct_delta(words: &[String]) -> i32 {
    if !matches!(
        words.first().map(String::as_str),
        Some("DATA" | "TYPES" | "CONSTANTS" | "STATICS" | "CLASS-DATA")
    ) {
        return 0;
    }
    words
        .windows(2)
        .map(|p| match (p[0].as_str(), p[1].as_str()) {
            ("BEGIN", "OF") => 1,
            ("END", "OF") => -1,
            _ => 0,
        })
        .sum()
}

/// Processing blocks that end an event block.
const ENDS_EVENT: &[&str] = &[
    "ENDFORM",
    "ENDMODULE",
    "ENDFUNCTION",
    "ENDCLASS",
    "ENDINTERFACE",
];

/// Blocks whose intermediates are indented one level and bodies two.
fn extra_level(words: &[String]) -> u32 {
    match words.first().map(String::as_str) {
        Some("CASE" | "TRY") => 1,
        Some("CLASS") if words.iter().any(|w| w == "DEFINITION") => 1,
        _ => 0,
    }
}

/// An open block: its closer, the opener's offset, the indent levels it adds,
/// and the levels of all open blocks up to and including it (so the current
/// level costs no walk over the stack, however deep the nesting).
type Block = (&'static str, u32, u32, u32);

fn push(blocks: &mut Vec<Block>, closer: &'static str, at: u32, levels: u32) {
    let total = blocks.last().map_or(0, |b| b.3) + levels;
    blocks.push((closer, at, levels, total));
}

pub fn check(text: &str, stream: &TokenStream) -> Result<Vec<Statement>, Diagnostic> {
    let tokens = &stream.tokens;
    let mut statements = Vec::new();
    let mut blocks: Vec<Block> = Vec::new();
    let mut brackets: Vec<(u16, u32)> = Vec::new(); // (closing kind, offset)
    let mut start = 0usize;
    // Dynpro flow logic starts with PROCESS; there `MODULE x.` calls a
    // module instead of defining one.
    let flow_logic = tokens
        .first()
        .is_some_and(|t| t.kind == WORD && t.text(text).eq_ignore_ascii_case("PROCESS"));

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
                // ABAP has no reserved words: `do( ).`, `if = 1.` or
                // `case->m( ).` are calls or assignments, never blocks.
                let keyword = keyword_position(text, tokens, start, k);
                let module_call = flow_logic && words.first().is_some_and(|w| w == "MODULE");
                let role = if !keyword || module_call {
                    None
                } else {
                    classify(&words, &depth0)
                };
                // An event block ends at the next event or processing block
                // on its level, and before a closer meant for an outer block.
                // `GET node` is an event only outside processing blocks;
                // inside one it is a statement such as `GET TIME`.
                let event = keyword
                    && is_event(&words)
                    && (words[0] != "GET" || blocks.last().is_none_or(|b| b.0 == EVENT));
                let ends_event = event
                    || matches!(role, Some(Role::Opener(c)) if ENDS_EVENT.contains(&c))
                    || role == Some(Role::Closer);
                if ends_event && blocks.last().is_some_and(|b| b.0 == EVENT) {
                    blocks.pop();
                }
                let delta = if keyword { struct_delta(&words) } else { 0 };
                if delta < 0 {
                    for _ in 0..-delta {
                        if blocks.last().is_some_and(|b| b.0 == END_OF) {
                            blocks.pop();
                        }
                    }
                }
                let level: u32 = blocks.last().map_or(0, |b| b.3);
                if event && blocks.is_empty() {
                    push(&mut blocks, EVENT, opener_at, 1);
                }
                for _ in 0..delta.max(0) {
                    push(&mut blocks, END_OF, opener_at, 1);
                }
                let depth_here = match role {
                    Some(Role::Opener(closer)) => {
                        push(&mut blocks, closer, opener_at, 1 + extra_level(&words));
                        level
                    }
                    Some(Role::Closer) => match blocks.pop() {
                        Some((closer, _, levels, _)) if closer == words[0] => level - levels,
                        Some((closer, _, _, _)) => {
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
                    role,
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
    while blocks.last().is_some_and(|b| b.0 == EVENT || b.0 == END_OF) {
        blocks.pop();
    }
    if let Some((closer, at, _, _)) = blocks.last() {
        return Err(err(
            Code::E004,
            format!("Missing {closer} for this block"),
            *at,
        ));
    }
    Ok(statements)
}

//! ABAP-SPACE-001: no space before a statement's period or a comma outside
//! brackets.
//!
//! The layout cannot remove a gap the input has (the builder keeps every
//! gap), so this pass prepares the token stream that the layout and the
//! printer see: each period or comma it glues is marked `glued`, and the
//! comments between it and the previous token are anchored behind it (an
//! end-of-line comment trails it, comment lines lead the next token). The
//! input stream only gets `may_glue`, for the self-check.

use crate::kinds::*;
use crate::structure::Statement;
use origami_syntax::{TokenIndex, TokenStream, TriviaPosition};

/// A period or comma that can be glued to the previous token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glue {
    /// Index of the period or comma.
    pub at: usize,
    /// Index of its statement.
    pub statement: usize,
    /// Comments stand between it and the previous token, so they move.
    pub moves: bool,
}

/// The periods and commas of `statements` that ABAP-SPACE-001 glues.
pub fn candidates(text: &str, stream: &TokenStream, statements: &[Statement]) -> Vec<Glue> {
    let toks = &stream.tokens;
    let mut leading = vec![false; toks.len() + 1];
    let mut trailing = vec![false; toks.len() + 1];
    for t in stream.trivia.iter().filter(|t| t.is_comment()) {
        match t.position {
            TriviaPosition::Leading => leading[t.anchor.0 as usize] = true,
            TriviaPosition::Trailing => trailing[t.anchor.0 as usize] = true,
        }
    }
    let mut out = Vec::new();
    for (id, st) in statements.iter().enumerate() {
        let r = st.tokens.clone();
        // The period of a FUNCTION statement stays behind the comment lines
        // in front of it (the generated signature comment, as ABAP Cleaner).
        let function = toks[r.start].kind == WORD
            && toks[r.start].text(text).eq_ignore_ascii_case("FUNCTION")
            && toks
                .get(r.start + 1)
                .is_some_and(|t| t.kind == WORD && t.text(text) != "=");
        let mut depth = 0i32;
        for k in r.clone() {
            let kind = toks[k].kind;
            match kind {
                LPAREN | LBRACKET | LBRACE | TEMPLATE_HEAD => depth += 1,
                RPAREN | RBRACKET | RBRACE | TEMPLATE_TAIL => depth -= 1,
                _ => {}
            }
            let comma = kind == COMMA && depth == 0;
            let period = kind == PERIOD && k + 1 == r.end;
            if !(comma || period) || k == r.start || toks[k].glued {
                continue;
            }
            let prev = &toks[k - 1];
            // `WRITE / .` keeps its space, as in ABAP Cleaner; an opaque body
            // is never touched.
            if prev.kind == OPAQUE || prev.text(text) == "/" {
                continue;
            }
            let moves = trailing[k - 1] || leading[k];
            // A period or comma with its own end-of-line comment cannot move
            // in front of other comments: it would end up with two.
            if moves && (trailing[k] || (period && function)) {
                continue;
            }
            out.push(Glue {
                at: k,
                statement: id,
                moves,
            });
        }
    }
    out
}

/// Marks each glued token and anchors the comments in front of it behind it.
pub fn apply(stream: &mut TokenStream, glue: &[Glue]) {
    let n = stream.tokens.len();
    let mut moved = vec![false; n + 1];
    for g in glue {
        stream.tokens[g.at].glued = true;
        moved[g.at] |= g.moves;
    }
    // Last comment line in front of each moved token: the line breaks after
    // it stay with the token, so they do not add a blank line before the
    // next one.
    let mut last_line = vec![usize::MAX; n + 1];
    for (k, t) in stream.trivia.iter().enumerate() {
        let a = t.anchor.0 as usize;
        if moved[a] && t.is_comment() && t.position == TriviaPosition::Leading {
            last_line[a] = k;
        }
    }
    for (k, t) in stream.trivia.iter_mut().enumerate() {
        let a = t.anchor.0 as usize;
        match t.position {
            // An end-of-line comment of the previous token.
            TriviaPosition::Trailing if a < n && moved[a + 1] => {
                t.anchor = TokenIndex(a as u32 + 1);
            }
            // Comment lines in front of the token, with the blank lines
            // between them.
            TriviaPosition::Leading
                if moved[a] && last_line[a] != usize::MAX && k <= last_line[a] =>
            {
                t.anchor = TokenIndex(a as u32 + 1)
            }
            _ => {}
        }
    }
}

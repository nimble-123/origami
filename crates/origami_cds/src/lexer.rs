//! Lossless lexer for the CDS family.
//!
//! Comments: `//` and `/* */` in all five languages; `--` only in DDLS, DDLX
//! and DCL (one flag per language, verified against the corpus at M2/M3).
//! Identifiers are `[A-Za-z_][A-Za-z0-9_]*` with optional namespace prefixes
//! (`/DMO/I_Travel`). `$name` and `#NAME` are single tokens. `.` is a separate
//! token between path steps.

use crate::kinds::*;
use origami_syntax::trivia::attach;
use origami_syntax::{
    Code, CommentSyntax, Diagnostic, Language, Placement, SourceText, Token, TokenStream, Trivia,
    TriviaKind,
};

/// Whether `--` starts a line comment in this language.
pub fn dash_comments(lang: Language) -> bool {
    matches!(lang, Language::Ddls | Language::Ddlx | Language::Dcl)
}

fn ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn ident_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

pub fn lex(src: &SourceText, lang: Language) -> Result<TokenStream, Diagnostic> {
    let text = src.text();
    let b = text.as_bytes();
    let dashes = dash_comments(lang);
    let mut tokens: Vec<Token> = Vec::new();
    let mut trivia: Vec<Trivia> = Vec::new();
    let mut i = 0usize;
    let mut gap = true;
    let mut token_on_line = false;
    let at = |k: usize| b.get(k).copied().unwrap_or(0);
    let e001 = |k: usize, m: &str| Diagnostic::new(Code::E001, m, k as u32..k as u32 + 1);

    while i < b.len() {
        let c = b[i];
        let push_comment = |syntax, end: usize, trivia: &mut Vec<Trivia>| {
            let placement = if token_on_line {
                Placement::EndOfLine
            } else {
                Placement::OwnLine
            };
            trivia.push(Trivia::new(
                TriviaKind::Comment { syntax, placement },
                i..end,
            ));
        };
        // Comments and whitespace.
        if c == b'\n' {
            trivia.push(Trivia::new(TriviaKind::Newline, i..i + 1));
            i += 1;
            gap = true;
            token_on_line = false;
            continue;
        }
        if matches!(c, b' ' | b'\t' | b'\r') {
            let s = i;
            while matches!(at(i), b' ' | b'\t' | b'\r') {
                i += 1;
            }
            trivia.push(Trivia::new(TriviaKind::Whitespace, s..i));
            gap = true;
            continue;
        }
        if (c == b'/' && at(i + 1) == b'/') || (dashes && c == b'-' && at(i + 1) == b'-') {
            let end = b[i..]
                .iter()
                .position(|&x| x == b'\n')
                .map(|p| i + p)
                .unwrap_or(b.len());
            push_comment(CommentSyntax::CdsLine, end, &mut trivia);
            i = end;
            gap = true;
            continue;
        }
        if c == b'/' && at(i + 1) == b'*' {
            let Some(p) = text[i + 2..].find("*/") else {
                return Err(e001(i, "Unterminated block comment"));
            };
            let end = i + 2 + p + 2;
            push_comment(CommentSyntax::CdsBlock, end, &mut trivia);
            i = end;
            gap = true;
            continue;
        }

        // Tokens.
        let (kind, end) = match c {
            b'\'' => {
                let mut j = i + 1;
                loop {
                    match at(j) {
                        b'\'' if at(j + 1) == b'\'' => j += 2,
                        b'\'' => break (STRING, j + 1),
                        b'\n' => return Err(e001(i, "Unterminated string")),
                        0 if j >= b.len() => return Err(e001(i, "Unterminated string")),
                        _ => j += 1,
                    }
                }
            }
            b'@' if at(i + 1) == b'<' => (AT_LT, i + 2),
            b'@' => (AT, i + 1),
            b'{' => (LBRACE, i + 1),
            b'}' => (RBRACE, i + 1),
            b'(' => (LPAREN, i + 1),
            b')' => (RPAREN, i + 1),
            b'[' => (LBRACKET, i + 1),
            b']' => (RBRACKET, i + 1),
            b';' => (SEMICOLON, i + 1),
            b',' => (COMMA, i + 1),
            b'.' if !at(i + 1).is_ascii_digit() => (DOT, i + 1),
            b':' => (COLON, i + 1),
            b'$' if ident_start(at(i + 1)) => (DOLLAR_IDENT, scan_ident(b, i + 1)),
            b'#' if ident_start(at(i + 1)) => (ENUM_VALUE, scan_ident(b, i + 1)),
            b'/' if namespace_len(b, i) > 0 => (IDENT, scan_ident(b, i + namespace_len(b, i))),
            _ if ident_start(c) => (IDENT, scan_ident(b, i)),
            _ if c.is_ascii_digit() || c == b'.' => {
                let mut j = i;
                while at(j).is_ascii_digit() || at(j) == b'.' {
                    j += 1;
                }
                (NUMBER, j)
            }
            _ => {
                // Keep multi-byte characters whole.
                let mut j = i + 1;
                while !text.is_char_boundary(j) {
                    j += 1;
                }
                (PUNCT, j)
            }
        };
        tokens.push(Token {
            kind,
            start: i as u32,
            end: end as u32,
            glued: !gap && !tokens.is_empty(),
            keyword: false,
        });
        i = end;
        gap = false;
        token_on_line = true;
    }
    attach(&tokens, &mut trivia);
    Ok(TokenStream { tokens, trivia })
}

fn scan_ident(b: &[u8], mut j: usize) -> usize {
    while j < b.len() && ident_char(b[j]) {
        j += 1;
    }
    j
}

/// Length of a namespace prefix `/NS/` at `i` that is followed by an
/// identifier, or 0. A `/` that is not followed by this pattern is division.
fn namespace_len(b: &[u8], i: usize) -> usize {
    let mut j = i + 1;
    if !b.get(j).copied().is_some_and(ident_start) {
        return 0;
    }
    while j < b.len() && ident_char(b[j]) {
        j += 1;
    }
    if b.get(j) == Some(&b'/') && b.get(j + 1).copied().is_some_and(ident_start) {
        j + 1 - i
    } else {
        0
    }
}

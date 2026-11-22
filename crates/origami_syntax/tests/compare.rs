use origami_syntax::trivia::attach;
use origami_syntax::{
    CommentSyntax, MismatchKind, Placement, Token, TokenStream, Trivia, TriviaKind, equivalent,
};

/// Minimal whitespace lexer for tests: words separated by spaces; `"` starts a
/// comment to end of line.
fn lex(src: &str) -> TokenStream {
    let mut tokens = Vec::new();
    let mut trivia = Vec::new();
    let b = src.as_bytes();
    let mut i = 0;
    let mut gap = true;
    let mut token_on_line = false;
    while i < b.len() {
        match b[i] {
            b' ' => {
                gap = true;
                i += 1;
            }
            b'\n' => {
                gap = true;
                token_on_line = false;
                i += 1;
            }
            b'"' => {
                let s = i;
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                let placement = if token_on_line {
                    Placement::EndOfLine
                } else {
                    Placement::OwnLine
                };
                trivia.push(Trivia::new(
                    TriviaKind::Comment {
                        syntax: CommentSyntax::AbapQuote,
                        placement,
                    },
                    s..i,
                ));
                gap = true;
            }
            _ => {
                let s = i;
                while i < b.len() && !matches!(b[i], b' ' | b'\n' | b'"') {
                    i += 1;
                }
                tokens.push(Token {
                    kind: 1,
                    start: s as u32,
                    end: i as u32,
                    glued: !gap && !tokens.is_empty(),
                    keyword: false,
                });
                gap = false;
                token_on_line = true;
            }
        }
    }
    attach(&tokens, &mut trivia);
    TokenStream { tokens, trivia }
}

fn kinds(a: &str, b: &str) -> Vec<MismatchKind> {
    match equivalent(&lex(a), a, &lex(b), b) {
        Ok(()) => vec![],
        Err(m) => m.into_iter().map(|m| m.kind).collect(),
    }
}

#[test]
fn whitespace_only_changes_are_equivalent() {
    assert!(kinds("a b\nc", "a    b c").is_empty());
}

#[test]
fn text_change_is_detected() {
    assert_eq!(kinds("a b", "a c"), vec![MismatchKind::Text]);
}

#[test]
fn keyword_case_is_exempt_only_for_keywords() {
    let a = "data x";
    let b = "DATA x";
    let mut input = lex(a);
    assert!(equivalent(&input, a, &lex(b), b).is_err());
    input.tokens[0].keyword = true;
    assert!(equivalent(&input, a, &lex(b), b).is_ok());
}

#[test]
fn count_change_is_detected() {
    assert_eq!(kinds("a b c", "a bc"), vec![MismatchKind::Count]);
}

#[test]
fn comment_text_change_is_detected() {
    assert_eq!(
        kinds("a \"x\nb", "a \"y\nb"),
        vec![MismatchKind::CommentText]
    );
}

#[test]
fn comment_moving_lines_is_detected() {
    // End-of-line comment after `a` becomes an own-line comment before `b`.
    assert_eq!(
        kinds("a \"x\nb", "a\n\"x\nb"),
        vec![MismatchKind::CommentKind]
    );
}

#[test]
fn comment_count_change_is_detected() {
    assert_eq!(kinds("a \"x\nb", "a\nb"), vec![MismatchKind::CommentText]);
}

#[test]
fn glued_change_is_detected() {
    // Same token count and texts, but `b` loses its gap: simulate by lexing
    // a stream where the second token is glued.
    let a = "a b";
    let b = "a b";
    let mut out = lex(b);
    out.tokens[1].glued = true;
    let ms = origami_syntax::equivalent(&lex(a), a, &out, b).unwrap_err();
    assert_eq!(ms[0].kind, MismatchKind::Glued);
}

#[test]
fn count_mismatch_points_past_keyword_case_changes() {
    // The output lost `importing b`; the upper-cased keyword before it is
    // not the difference.
    let (a, b) = ("methods x importing b", "METHODS x");
    let mut input = lex(a);
    input.tokens[0].keyword = true;
    let ms = equivalent(&input, a, &lex(b), b).unwrap_err();
    assert_eq!(ms.len(), 1);
    assert_eq!(ms[0].kind, MismatchKind::Count);
    assert_eq!(ms[0].input_index.0, 2);
}

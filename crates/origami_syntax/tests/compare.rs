use origami_syntax::trivia::attach;
use origami_syntax::{
    CommentSyntax, MismatchKind, Placement, Token, TokenStream, Trivia, TriviaKind, equivalent,
};

/// Minimal whitespace lexer for tests: words separated by spaces, `.` and `,`
/// are tokens of their own; `"` starts a comment to end of line (`"!` an ABAP
/// Doc comment).
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
                let syntax = if src[s..].starts_with("\"!") {
                    CommentSyntax::AbapDoc
                } else {
                    CommentSyntax::AbapQuote
                };
                trivia.push(Trivia::new(TriviaKind::Comment { syntax, placement }, s..i));
                gap = true;
            }
            _ => {
                let s = i;
                i += 1;
                while i < b.len()
                    && !matches!(b[s], b'.' | b',')
                    && !matches!(b[i], b' ' | b'\n' | b'"' | b'.' | b',')
                {
                    i += 1;
                }
                tokens.push(Token {
                    kind: 1,
                    start: s as u32,
                    end: i as u32,
                    glued: !gap && !tokens.is_empty(),
                    keyword: false,
                    ident: false,
                    may_glue: false,
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
fn identifier_case_is_exempt_only_for_identifiers() {
    let a = "DATA LV_X";
    let b = "DATA lv_x";
    let mut input = lex(a);
    assert!(equivalent(&input, a, &lex(b), b).is_err());
    input.tokens[1].ident = true;
    assert!(equivalent(&input, a, &lex(b), b).is_ok());
    // Any other text change is still a mismatch.
    let c = "DATA lv_y";
    assert!(equivalent(&input, a, &lex(c), c).is_err());
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

/// Like `kinds`, with `may_glue` set on input tokens `marked`.
fn glue_kinds(a: &str, b: &str, marked: &[usize]) -> Vec<MismatchKind> {
    let mut input = lex(a);
    for &k in marked {
        input.tokens[k].may_glue = true;
    }
    match equivalent(&input, a, &lex(b), b) {
        Ok(()) => vec![],
        Err(m) => m.into_iter().map(|m| m.kind).collect(),
    }
}

#[test]
fn marked_period_and_comma_may_become_glued() {
    assert!(glue_kinds("a , b .", "a, b.", &[1, 3]).is_empty());
    // Without the mark, the same change is a mismatch.
    assert_eq!(
        glue_kinds("a , b .", "a, b.", &[1]),
        vec![MismatchKind::Glued]
    );
}

#[test]
fn only_marked_tokens_may_become_glued() {
    // The token after the comma may not touch it, and a glued token may not
    // be separated.
    assert_eq!(glue_kinds("a , b", "a ,b", &[1]), vec![MismatchKind::Glued]);
    assert_eq!(glue_kinds("a.", "a .", &[1]), vec![MismatchKind::Glued]);
}

#[test]
fn comments_move_behind_a_glued_period_or_comma() {
    // End-of-line comment of the previous token, then a comment line.
    assert!(glue_kinds("a \"x\n\"y\n.\nb", "a. \"x\n\"y\nb", &[1]).is_empty());
    assert!(glue_kinds("a \"x\n, b", "a, \"x\nb", &[1]).is_empty());
}

#[test]
fn comments_move_only_with_a_glued_marked_token() {
    let a = "a \"x\n.\nb";
    // Not marked.
    assert!(glue_kinds(a, "a. \"x\nb", &[]).contains(&MismatchKind::CommentAnchor));
    // Marked, but not glued in the output.
    assert_eq!(
        glue_kinds(a, "a\n. \"x\nb", &[1]),
        vec![MismatchKind::CommentAnchor]
    );
}

#[test]
fn comments_move_by_one_token_only() {
    // Two tokens further.
    assert_eq!(
        glue_kinds("a \"x\n. b c", "a. b \"x\nc", &[1]),
        vec![MismatchKind::CommentAnchor]
    );
    assert_eq!(
        glue_kinds("a\n\"x\n. b c", "a. b\n\"x\nc", &[1]),
        vec![MismatchKind::CommentAnchor]
    );
    // A comment that did not stand in front of the glued token stays.
    assert_eq!(
        glue_kinds("a \"x\nb .", "a b. \"x", &[2]),
        vec![MismatchKind::CommentAnchor]
    );
    // Nor may a comment change its placement on the way.
    assert_eq!(
        glue_kinds("a \"x\n.\nb", "a.\n\"x\nb", &[1]),
        vec![MismatchKind::CommentKind]
    );
}

#[test]
fn abap_doc_tag_lines_may_change_blanks_after_tag_and_before_bar() {
    let ok = |a: &str, b: &str| kinds(&format!("{a}\nm"), &format!("{b}\nm")).is_empty();
    assert!(ok("\"! @parameter a | x", "\"! @parameter a    | x"));
    assert!(ok("\"! @raising a| x", "\"! @raising   a | x"));
    assert!(ok("\"! @EXCEPTION a\t| x", "\"! @EXCEPTION   a  | x"));
    assert!(ok("\"! @parameter    a | x", "\"! @parameter a | x"));
}

#[test]
fn abap_doc_lines_keep_every_other_byte() {
    let bad = |a: &str, b: &str| {
        kinds(&format!("{a}\nm"), &format!("{b}\nm")) == vec![MismatchKind::CommentText]
    };
    // After the bar, before the tag, the name, the tag itself.
    assert!(bad("\"! @parameter a | x", "\"! @parameter a |  x"));
    assert!(bad("\"! @parameter a | x", "\"!  @parameter a | x"));
    assert!(bad("\"! @parameter a | x", "\"! @parameter b | x"));
    assert!(bad("\"! @parameter a | x", "\"! @PARAMETER a | x"));
    // The blank after the tag stays, and the name stays one word.
    assert!(bad("\"! @parameter a | x", "\"! @parametera | x"));
    assert!(bad("\"! @parameter ab | x", "\"! @parameter a b | x"));
    // Other tags, lines without a bar, and comments that are not ABAP Doc.
    assert!(bad("\"! @see a | x", "\"! @see   a | x"));
    assert!(bad("\"! @parameter a x", "\"! @parameter   a x"));
    assert!(bad("\" @parameter a | x", "\" @parameter   a | x"));
}

use origami_syntax::trivia::attach;
use origami_syntax::{
    CommentSyntax, Placement, Token, TokenIndex, Trivia, TriviaKind, TriviaPosition,
};

fn tok(start: u32, end: u32) -> Token {
    Token {
        kind: 1,
        start,
        end,
        glued: false,
        keyword: false,
        ident: false,
    }
}

fn comment(start: usize, end: usize, placement: Placement) -> Trivia {
    Trivia::new(
        TriviaKind::Comment {
            syntax: CommentSyntax::AbapQuote,
            placement,
        },
        start..end,
    )
}

#[test]
fn end_of_line_comment_trails_previous_token() {
    let tokens = vec![tok(0, 4), tok(10, 14)];
    let mut trivia = vec![comment(5, 9, Placement::EndOfLine)];
    attach(&tokens, &mut trivia);
    assert_eq!(trivia[0].anchor, TokenIndex(0));
    assert_eq!(trivia[0].position, TriviaPosition::Trailing);
}

#[test]
fn own_line_comment_leads_next_token() {
    let tokens = vec![tok(0, 4), tok(10, 14)];
    let mut trivia = vec![comment(5, 9, Placement::OwnLine)];
    attach(&tokens, &mut trivia);
    assert_eq!(trivia[0].anchor, TokenIndex(1));
    assert_eq!(trivia[0].position, TriviaPosition::Leading);
}

#[test]
fn trailing_own_line_comment_uses_eof_anchor() {
    let tokens = vec![tok(0, 4)];
    let mut trivia = vec![comment(5, 9, Placement::OwnLine)];
    attach(&tokens, &mut trivia);
    assert_eq!(trivia[0].anchor, TokenIndex(1));
}

#[test]
fn leading_comment_before_first_token() {
    let tokens = vec![tok(10, 14)];
    let mut trivia = vec![comment(0, 9, Placement::OwnLine)];
    attach(&tokens, &mut trivia);
    assert_eq!(trivia[0].anchor, TokenIndex(0));
    assert_eq!(trivia[0].position, TriviaPosition::Leading);
}

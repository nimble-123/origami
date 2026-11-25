//! Printer unit tests: groups, hanging indentation, choice, fill, alignment,
//! comments, blank lines, identity mode, and linear run time.

use origami_layout::builder as b;
use origami_layout::{Doc, PrintOptions, print};
use origami_syntax::trivia::attach;
use origami_syntax::{
    Case, CommentSyntax, Placement, SourceText, Token, TokenIndex, TokenStream, Trivia, TriviaKind,
};

/// Words separated by spaces become tokens (glued when no space before);
/// `"` starts a comment to the end of the line; `*` in column 1 is a star
/// comment.
fn lex(src: &str) -> TokenStream {
    let mut tokens = Vec::new();
    let mut trivia = Vec::new();
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut gap = true;
    let mut on_line = false;
    while i < bytes.len() {
        let c = bytes[i];
        let col0 = i == 0 || bytes[i - 1] == b'\n';
        if c == b'\n' {
            trivia.push(Trivia::new(TriviaKind::Newline, i..i + 1));
            i += 1;
            gap = true;
            on_line = false;
        } else if c == b' ' {
            let s = i;
            while i < bytes.len() && bytes[i] == b' ' {
                i += 1;
            }
            trivia.push(Trivia::new(TriviaKind::Whitespace, s..i));
            gap = true;
        } else if c == b'"' || (c == b'*' && col0) {
            let s = i;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            let syntax = if c == b'*' {
                CommentSyntax::AbapStar
            } else {
                CommentSyntax::AbapQuote
            };
            let placement = if on_line {
                Placement::EndOfLine
            } else {
                Placement::OwnLine
            };
            trivia.push(Trivia::new(TriviaKind::Comment { syntax, placement }, s..i));
            gap = true;
        } else {
            let s = i;
            if c == b'(' {
                i += 1;
            } else {
                while i < bytes.len() && !matches!(bytes[i], b' ' | b'\n' | b'"' | b'(') {
                    i += 1;
                }
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
            on_line = true;
        }
    }
    attach(&tokens, &mut trivia);
    TokenStream { tokens, trivia }
}

fn opts(width: u16) -> PrintOptions {
    PrintOptions {
        width,
        indent: 2,
        keyword_case: Some(Case::Upper),
        identity: false,
        cont_levels: 2,
        comment_overrides: Default::default(),
    }
}

fn t(i: u32) -> Doc {
    b::token(TokenIndex(i))
}

fn render(src: &str, width: u16, f: impl Fn(&TokenStream) -> Doc) -> String {
    let ts = lex(src);
    let doc = f(&ts);
    print(&doc, &SourceText::from_normalized(src), &ts, &opts(width))
}

/// `t0 sep t1 sep t2 …` inside one group with soft lines.
fn words_group(ts: &TokenStream) -> Doc {
    let n = ts.tokens.len() as u32;
    let mut docs = vec![t(0)];
    for k in 1..n {
        docs.push(b::soft(ts, TokenIndex(k)));
        docs.push(t(k));
    }
    b::group(docs)
}

#[test]
fn identity_mode_returns_input() {
    let src = "a   b\n  c\n";
    let ts = lex(src);
    let mut o = opts(80);
    o.identity = true;
    assert_eq!(
        print(&b::empty(), &SourceText::from_normalized(src), &ts, &o),
        src
    );
}

#[test]
fn group_is_flat_when_it_fits_and_broken_otherwise() {
    assert_eq!(render("aa bb cc", 20, words_group), "aa bb cc\n");
    assert_eq!(render("aa bb cc", 5, words_group), "aa\nbb\ncc\n");
}

#[test]
fn glued_tokens_get_no_gap() {
    // `b` is glued to `a`; the builder emits nothing before it.
    // Tokens: a ( b c — `(` and `b` are glued.
    let out = render("a(b c", 80, |ts| {
        b::concat(vec![
            t(0),
            b::keep(ts, TokenIndex(1)),
            t(1),
            b::keep(ts, TokenIndex(2)),
            t(2),
            b::sep(ts, TokenIndex(3)),
            t(3),
        ])
    });
    assert_eq!(out, "a(b c\n");
}

#[test]
#[should_panic(expected = "glued")]
fn adding_a_gap_before_a_glued_token_fails_in_debug() {
    render("a(b", 80, |ts| {
        b::concat(vec![t(0), b::sep(ts, TokenIndex(1)), t(1)])
    });
}

#[test]
fn keyword_case_applies_only_to_keywords() {
    let src = "data x";
    let mut ts = lex(src);
    ts.tokens[0].keyword = true;
    let doc = b::concat(vec![t(0), b::sep(&ts, TokenIndex(1)), t(1)]);
    assert_eq!(
        print(&doc, &SourceText::from_normalized(src), &ts, &opts(80)),
        "DATA x\n"
    );
}

#[test]
fn hang_continues_at_the_current_column() {
    // Tokens: call ( aa bb cc ) — `(` is glued to `call`.
    let out = render("call( aa bb cc )", 12, |ts| {
        b::concat(vec![
            t(0),
            b::keep(ts, TokenIndex(1)),
            t(1),
            b::sep(ts, TokenIndex(2)),
            b::hang(vec![b::group(vec![
                t(2),
                b::soft(ts, TokenIndex(3)),
                t(3),
                b::soft(ts, TokenIndex(4)),
                t(4),
            ])]),
            b::sep(ts, TokenIndex(5)),
            t(5),
        ])
    });
    assert_eq!(out, "call( aa\n      bb\n      cc )\n");
}

#[test]
fn choice_takes_the_second_doc_when_the_first_overflows() {
    let out = render("xxxxxxxxxx yy", 8, |ts| {
        let first = b::concat(vec![t(0), b::sep(ts, TokenIndex(1)), t(1)]);
        let second = b::concat(vec![t(0), b::hard(ts, TokenIndex(1)), t(1)]);
        b::choice(first, second)
    });
    assert_eq!(out, "xxxxxxxxxx\nyy\n");
}

#[test]
fn choice_after_a_line_break_counts_the_indentation_once() {
    // `yyyy z` at indent 2 ends at column 8: the first doc fits.
    let out = render("x yyyy z", 8, |ts| {
        let first = b::concat(vec![t(1), b::sep(ts, TokenIndex(2)), t(2)]);
        let second = b::concat(vec![t(1), b::hard(ts, TokenIndex(2)), t(2)]);
        b::concat(vec![
            t(0),
            b::indent(vec![b::hard(ts, TokenIndex(1)), b::choice(first, second)]),
        ])
    });
    assert_eq!(out, "x\n  yyyy z\n");
}

#[test]
fn fill_breaks_only_before_parts_that_do_not_fit() {
    let out = render("head aa bbbbbbbbbb cc", 14, |ts| {
        b::concat(vec![
            t(0),
            b::fill(vec![
                (b::soft(ts, TokenIndex(1)), t(1)),
                (b::soft(ts, TokenIndex(2)), t(2)),
                (b::soft(ts, TokenIndex(3)), t(3)),
            ]),
        ])
    });
    assert_eq!(out, "head aa\nbbbbbbbbbb cc\n");
}

fn rows(ts: &TokenStream, pairs: &[(u32, u32, u32)]) -> Doc {
    // (name, `=`, value) token indices
    let items = pairs
        .iter()
        .map(|&(n, eq, v)| {
            b::row(
                vec![t(n)],
                vec![
                    b::sep(ts, TokenIndex(eq)),
                    t(eq),
                    b::sep(ts, TokenIndex(v)),
                    t(v),
                ],
            )
        })
        .collect();
    b::align(items)
}

#[test]
fn alignment_pads_heads_in_broken_mode() {
    let out = render("a = 1 bbb = 2", 8, |ts| {
        b::group(vec![rows(ts, &[(0, 1, 2), (3, 4, 5)])])
    });
    assert_eq!(out, "a   = 1\nbbb = 2\n");
}

#[test]
fn alignment_is_flat_when_it_fits() {
    let out = render("a = 1 bbb = 2", 80, |ts| {
        b::group(vec![rows(ts, &[(0, 1, 2), (3, 4, 5)])])
    });
    assert_eq!(out, "a = 1 bbb = 2\n");
}

#[test]
fn alignment_falls_back_to_single_spaces_on_overflow() {
    let out = render("a = 1 bbbbbbbbbbbb = 2", 14, |ts| {
        b::group(vec![rows(ts, &[(0, 1, 2), (3, 4, 5)])])
    });
    // Padding `a` to the width of the long head would exceed 14 columns.
    assert_eq!(out, "a = 1\nbbbbbbbbbbbb = 2\n");
}

/// Rows `(name, op, value)` as whole lines: anchors right-aligned by their
/// width, `KeepLines` fallback.
fn statement_rows(ts: &TokenStream, src: &str, pairs: &[(u32, u32, u32)]) -> Doc {
    let words: Vec<&str> = src.split_whitespace().collect();
    let items = pairs
        .iter()
        .map(|&(n, op, v)| {
            b::row_right(
                vec![t(n)],
                words[op as usize].len(),
                vec![
                    b::sep(ts, TokenIndex(op)),
                    b::group(vec![t(op), b::sep(ts, TokenIndex(v)), t(v)]),
                ],
            )
        })
        .collect();
    b::align_lines(items)
}

#[test]
fn right_aligned_anchors_share_their_last_column() {
    let src = "aaaa += 1 b = 2 cc -= 3";
    let out = render(src, 80, |ts| {
        statement_rows(ts, src, &[(0, 1, 2), (3, 4, 5), (6, 7, 8)])
    });
    assert_eq!(
        out,
        "aaaa += 1
b     = 2
cc   -= 3
"
    );
}

#[test]
fn unaligned_lines_keep_their_own_line_inside_a_group() {
    let src = "a = 1 other bbb = 2";
    let out = render(src, 80, |ts| {
        b::align_lines(vec![
            b::row(
                vec![t(0)],
                vec![
                    b::sep(ts, TokenIndex(1)),
                    t(1),
                    b::sep(ts, TokenIndex(2)),
                    t(2),
                ],
            ),
            b::line(vec![t(3)]),
            b::row(
                vec![t(4)],
                vec![
                    b::sep(ts, TokenIndex(5)),
                    t(5),
                    b::sep(ts, TokenIndex(6)),
                    t(6),
                ],
            ),
        ])
    });
    assert_eq!(
        out,
        "a   = 1
other
bbb = 2
"
    );
}

#[test]
fn keep_lines_alignment_never_pushes_a_fitting_row_past_the_width() {
    // `a = 1234567` fits into 12 columns, padded to `a      = 1234567` it
    // would not: the group keeps single spaces.
    let src = "a = 1234567 bbbbbb = 2";
    let out = render(src, 12, |ts| {
        statement_rows(ts, src, &[(0, 1, 2), (3, 4, 5)])
    });
    assert_eq!(
        out,
        "a = 1234567
bbbbbb = 2
"
    );
    // A row that is too wide anyway does not prevent alignment.
    let src = "a = 1234567890123 bbbbbb = 2";
    let out = render(src, 12, |ts| {
        statement_rows(ts, src, &[(0, 1, 2), (3, 4, 5)])
    });
    assert_eq!(
        out,
        "a      = 1234567890123
bbbbbb = 2
"
    );
}

#[test]
fn end_of_line_comment_forces_a_break() {
    let out = render("aa \"c\nbb", 80, words_group);
    assert_eq!(out, "aa \"c\nbb\n");
}

#[test]
fn star_comment_stays_in_column_one() {
    let out = render("* top\n   x", 80, |ts| {
        b::indent(vec![b::hard(ts, TokenIndex(0)), t(0)])
    });
    assert_eq!(out, "\n* top\n  x\n");
}

#[test]
fn blank_lines_are_kept_once_only_where_allowed() {
    let src = "a\n\n\n\nb\n\nc";
    let out = render(src, 80, |ts| {
        b::concat(vec![
            t(0),
            b::hard(ts, TokenIndex(1)),
            b::token_keep_blank(TokenIndex(1)),
            b::hard(ts, TokenIndex(2)),
            t(2),
        ])
    });
    assert_eq!(out, "a\n\nb\nc\n");
}

#[test]
fn unknown_units_are_reindented_and_spaces_shrink() {
    let src = "  aa    bb\n      cc";
    let out = render(src, 80, |_| b::unknown(TokenIndex(0), TokenIndex(2), false));
    // Shift -2: `aa` moves to column 0, `cc` keeps its relative indent.
    assert_eq!(out, "aa bb\n    cc\n");
}

#[test]
fn verbatim_units_keep_inner_spacing() {
    let src = "  aa    bb";
    let out = render(src, 80, |_| {
        b::verbatim(TokenIndex(0), TokenIndex(1), false)
    });
    assert_eq!(out, "aa    bb\n");
}

/// Catches quadratic printing, not small slowdowns: 5x the input should take
/// about 5x the time (a quadratic printer: 25x). Shared CI runners are noisy,
/// so each size is printed several times and the fastest run counts, and the
/// bound leaves room for noise while staying far below 25.
#[test]
fn printing_is_linear_in_size() {
    let make = |n: usize| {
        (0..n)
            .map(|k| format!("w{k}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let time = |n: usize| {
        let src = make(n);
        let ts = lex(&src);
        let mut docs = vec![t(0)];
        for k in 1..ts.tokens.len() as u32 {
            docs.push(b::hard(&ts, TokenIndex(k)));
            docs.push(b::group(vec![t(k)]));
        }
        let doc = b::concat(docs);
        let text = SourceText::from_normalized(src.clone());
        let o = opts(120);
        (0..5)
            .map(|_| {
                let start = std::time::Instant::now();
                let _ = print(&doc, &text, &ts, &o);
                start.elapsed().as_secs_f64()
            })
            .fold(f64::INFINITY, f64::min)
    };
    let _ = time(1_000); // warm up
    let small = time(10_000);
    let large = time(50_000);
    assert!(large / small < 12.0, "50k/10k time ratio {}", large / small);
}

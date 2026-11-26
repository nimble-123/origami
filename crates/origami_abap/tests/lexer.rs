use origami_abap::kinds::*;
use origami_abap::{lex, structure};
use origami_syntax::{Code, CommentSyntax, Placement, SourceText, TokenStream, TriviaKind};

fn toks(src: &str) -> (TokenStream, String) {
    let s = SourceText::from_normalized(src);
    (lex(&s).expect("lexes"), src.to_string())
}

/// Token texts, with `~` marking a glued token.
fn shape(src: &str) -> Vec<String> {
    let (ts, text) = toks(src);
    ts.tokens
        .iter()
        .map(|t| format!("{}{}", if t.glued { "~" } else { "" }, t.text(&text)))
        .collect()
}

#[test]
fn glued_flags() {
    assert_eq!(shape("a-b."), ["a-b", "~."]);
    assert_eq!(shape("a - b."), ["a", "-", "b", "~."]);
    assert_eq!(
        shape("x = lv+off(len)."),
        ["x", "=", "lv+off", "~(", "~len", "~)", "~."]
    );
    assert_eq!(shape("meth( a )."), ["meth", "~(", "a", ")", "~."]);
    assert_eq!(
        shape("DATA(x) = 1."),
        ["DATA", "~(", "~x", "~)", "=", "1", "~."]
    );
    assert_eq!(
        shape("SELECT * FROM t INTO @lv."),
        ["SELECT", "*", "FROM", "t", "INTO", "@", "~lv", "~."]
    );
    assert_eq!(
        shape("x = itab[ 1 ]."),
        ["x", "=", "itab", "~[", "1", "]", "~."]
    );
    assert_eq!(
        shape("WRITE 'text'(001)."),
        ["WRITE", "'text'", "~(", "~001", "~)", "~."]
    );
}

#[test]
fn word_forms() {
    assert_eq!(
        shape("NEW /dmo/cl_flight( )."),
        ["NEW", "/dmo/cl_flight", "~(", ")", "~."]
    );
    assert_eq!(shape("x = ls-%cid."), ["x", "=", "ls-%cid", "~."]);
    assert_eq!(
        shape("ASSIGN x TO <fs>."),
        ["ASSIGN", "x", "TO", "<fs>", "~."]
    );
    assert_eq!(
        shape("METHODS m IMPORTING !iv TYPE i."),
        ["METHODS", "m", "IMPORTING", "!iv", "TYPE", "i", "~."]
    );
    assert_eq!(shape("obj->m( )."), ["obj", "~->", "~m", "~(", ")", "~."]);
    assert_eq!(shape("cl=>m( )."), ["cl", "~=>", "~m", "~(", ")", "~."]);
    assert_eq!(shape("intf~m( )."), ["intf", "~~", "~m", "~(", ")", "~."]);
    assert_eq!(
        shape("x = get( )->y."),
        ["x", "=", "get", "~(", ")", "~->", "~y", "~."]
    );
    assert_eq!(
        shape("x = VALUE #( )."),
        ["x", "=", "VALUE", "#", "~(", ")", "~."]
    );
}

#[test]
fn comments() {
    let src = "* full line\nDATA x. \" end of line\n\"! doc\nDATA y. \"#EC NEEDED\n";
    let (ts, _) = toks(src);
    let kinds: Vec<_> = ts
        .trivia
        .iter()
        .filter_map(|t| match t.kind {
            TriviaKind::Comment { syntax, placement } => Some((syntax, placement)),
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds,
        [
            (CommentSyntax::AbapStar, Placement::OwnLine),
            (CommentSyntax::AbapQuote, Placement::EndOfLine),
            (CommentSyntax::AbapDoc, Placement::OwnLine),
            (CommentSyntax::AbapPseudo, Placement::EndOfLine),
        ]
    );
}

#[test]
fn star_not_in_column_one_is_a_word() {
    assert_eq!(shape("x = a * b."), ["x", "=", "a", "*", "b", "~."]);
}

#[test]
fn pragmas() {
    assert_eq!(shape("DATA x ##NEEDED."), ["DATA", "x", "##NEEDED", "~."]);
    assert_eq!(
        shape("DATA x ##SHADOW[TEXT]."),
        ["DATA", "x", "##SHADOW[TEXT]", "~."]
    );
}

#[test]
fn templates() {
    let (ts, _) = toks("x = |a{ b }c{ |d{ e }| }f|.");
    let kinds: Vec<u16> = ts.tokens.iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        [
            WORD,
            WORD,
            TEMPLATE_HEAD,
            WORD,
            TEMPLATE_MIDDLE,
            TEMPLATE_HEAD,
            WORD,
            TEMPLATE_TAIL,
            TEMPLATE_TAIL,
            PERIOD
        ]
    );
    assert_eq!(shape("x = |a\\|b\\{c|."), ["x", "=", "|a\\|b\\{c|", "~."]);
}

#[test]
fn string_literals_with_escapes() {
    assert_eq!(shape("x = 'it''s'."), ["x", "=", "'it''s'", "~."]);
    assert_eq!(shape("x = `a``b`."), ["x", "=", "`a``b`", "~."]);
}

/// A `'` inside a template or a backquote literal does not open a literal in
/// a macro body, so `END-OF-DEFINITION.` is still found.
#[test]
fn macro_body_skips_templates_and_backquote_literals() {
    let bodies = [
        "lv = |it's &1|.",
        "lv = `it's &1`.",
        "lv = |a { 'x|' } b|.",
        "lv = |a { `x|` && |y'| } b|.",
        "lv = |a \\| it's|.",
        "lv = |a \\{ it's|.",
    ];
    for body in bodies {
        let src = format!("DEFINE m.\n  {body}\nEND-OF-DEFINITION.\nm x.\n");
        let (ts, text) = toks(&src);
        let opaque: Vec<_> = ts.tokens.iter().filter(|t| t.kind == OPAQUE).collect();
        assert_eq!(opaque.len(), 1, "{body}");
        assert!(opaque[0].text(&text).contains(body), "{body}");
        assert!(
            !opaque[0].text(&text).contains("END-OF-DEFINITION"),
            "{body}"
        );
    }
}

#[test]
fn verbatim_regions() {
    let src = "METHOD m BY DATABASE PROCEDURE FOR HDB LANGUAGE SQLSCRIPT.\n  -- endmethod. in comment\n  x = \"ENDMETHOD\";\n  ENDMETHOD.\n";
    let (ts, text) = toks(src);
    let opaque: Vec<_> = ts.tokens.iter().filter(|t| t.kind == OPAQUE).collect();
    assert_eq!(opaque.len(), 1);
    assert!(opaque[0].text(&text).contains("x = \"ENDMETHOD\";"));
    assert!(!opaque[0].text(&text).contains("ENDMETHOD."));

    let macro_src = "DEFINE m.\n  WRITE &1. \" END-OF-DEFINITION.\nEND-OF-DEFINITION.\n";
    let (ts, _) = toks(macro_src);
    assert_eq!(ts.tokens.iter().filter(|t| t.kind == OPAQUE).count(), 1);

    let exec = "EXEC SQL.\n  SELECT 1 FROM dummy\nENDEXEC.\n";
    let (ts, _) = toks(exec);
    assert_eq!(ts.tokens.iter().filter(|t| t.kind == OPAQUE).count(), 1);
}

#[test]
fn commented_out_opener_does_not_match() {
    let src = "* METHOD m BY DATABASE PROCEDURE.\nDATA x.\n";
    let (ts, _) = toks(src);
    assert!(ts.tokens.iter().all(|t| t.kind != OPAQUE));
}

#[test]
fn e001_cases() {
    for src in ["x = 'abc\n.", "x = `abc", "x = |abc", "x = |a{ b ."] {
        let err = lex(&SourceText::from_normalized(src)).unwrap_err();
        assert_eq!(err.code, Code::E001, "{src}");
    }
}

#[test]
fn missing_amdp_closer_is_e004() {
    let err = lex(&SourceText::from_normalized(
        "METHOD m BY DATABASE PROCEDURE.\n x;\n",
    ))
    .unwrap_err();
    assert_eq!(err.code, Code::E004);
}

fn check(src: &str) -> Result<Vec<structure::Statement>, Code> {
    let (ts, text) = toks(src);
    structure::check(&text, &ts).map_err(|d| d.code)
}

#[test]
fn structure_errors() {
    assert_eq!(check("x = f( a .").unwrap_err(), Code::E002);
    assert_eq!(check("x = a )."), Err(Code::E002));
    assert_eq!(check("DATA x"), Err(Code::E003));
    assert_eq!(check("IF x = 1.\nENDLOOP."), Err(Code::E004));
    assert_eq!(check("IF x = 1."), Err(Code::E004));
    assert_eq!(check("ENDIF."), Err(Code::E004));
}

#[test]
fn structure_accepts_valid_blocks() {
    let src = "CLASS lcl DEFINITION DEFERRED.\nCLASS lcl DEFINITION.\n  PUBLIC SECTION.\nENDCLASS.\n\
               CLASS lcl IMPLEMENTATION.\n  METHOD m.\n    IF a = 1.\n    ELSEIF a = 2.\n    ELSE.\n    ENDIF.\n\
               SELECT SINGLE * FROM t INTO @DATA(wa).\n    SELECT * FROM t INTO @wa.\n    ENDSELECT.\n\
               TRY.\n    CATCH cx_root.\n    ENDTRY.\n  ENDMETHOD.\nENDCLASS.\nAT SELECTION-SCREEN.\n";
    let st = check(src).expect("valid");
    let depths: Vec<u32> = st.iter().map(|s| s.depth).collect();
    // CLASS DEFINITION: section at 1; METHOD body at 2; TRY body at 4, CATCH at 3.
    assert_eq!(
        depths,
        [0, 0, 1, 0, 0, 1, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 0, 0]
    );
}

#[test]
fn keywords_used_as_names_are_not_blocks() {
    assert!(check("do( ).\nif = 1.\ncase->m( ).\nloop[ 1 ]-x = 2.\nwhile += 1.\n").is_ok());
}

#[test]
fn chained_statement_is_one_unit() {
    let st = check("DATA: a TYPE i,\n      b TYPE i.\n").unwrap();
    assert_eq!(st.len(), 1);
}

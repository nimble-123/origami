use origami_cds::kinds::*;
use origami_cds::{lex, structure};
use origami_syntax::{Code, CommentSyntax, Language, SourceText, TriviaKind};

fn shape(src: &str, lang: Language) -> Vec<String> {
    let ts = lex(&SourceText::from_normalized(src), lang).expect("lexes");
    ts.tokens
        .iter()
        .map(|t| format!("{}{}", if t.glued { "~" } else { "" }, t.text(src)))
        .collect()
}

fn comment_count(src: &str, lang: Language) -> usize {
    let ts = lex(&SourceText::from_normalized(src), lang).unwrap();
    ts.trivia.iter().filter(|t| t.is_comment()).count()
}

#[test]
fn comment_forms_per_language() {
    for lang in [
        Language::Ddls,
        Language::Ddlx,
        Language::Dcl,
        Language::Bdef,
        Language::Srvd,
    ] {
        assert_eq!(comment_count("a // x\nb /* y */ c", lang), 2, "{lang:?}");
    }
    for lang in [Language::Ddls, Language::Ddlx, Language::Dcl] {
        assert_eq!(comment_count("a -- x\nb", lang), 1, "{lang:?}");
    }
    for lang in [Language::Bdef, Language::Srvd] {
        assert_eq!(comment_count("a -- x\nb", lang), 0, "{lang:?}");
    }
}

#[test]
fn block_comment_syntax() {
    let ts = lex(
        &SourceText::from_normalized("/* a\n b */ x"),
        Language::Ddls,
    )
    .unwrap();
    assert!(matches!(
        ts.trivia[0].kind,
        TriviaKind::Comment {
            syntax: CommentSyntax::CdsBlock,
            ..
        }
    ));
}

#[test]
fn identifiers_and_special_tokens() {
    assert_eq!(
        shape(
            "select from /DMO/I_Travel as t { key t.TravelId, $projection.x, #TEXT }",
            Language::Ddls
        ),
        [
            "select",
            "from",
            "/DMO/I_Travel",
            "as",
            "t",
            "{",
            "key",
            "t",
            "~.",
            "~TravelId",
            "~,",
            "$projection",
            "~.",
            "~x",
            "~,",
            "#TEXT",
            "}"
        ]
    );
    assert_eq!(shape("a / b", Language::Ddls), ["a", "/", "b"]);
    assert_eq!(
        shape("@UI.lineItem: [{ position: 10 }]", Language::Ddlx)[..3],
        ["@", "~UI", "~."]
    );
    assert_eq!(shape("x @<Semantics.text: true", Language::Ddls)[1], "@<");
}

#[test]
fn association_paths() {
    assert_eq!(
        shape("_Agency[1: inner].Name", Language::Ddls),
        ["_Agency", "~[", "~1", "~:", "inner", "~]", "~.", "~Name"]
    );
}

#[test]
fn kinds_of_literals() {
    let src = "'it''s' 12.5";
    let ts = lex(&SourceText::from_normalized(src), Language::Ddls).unwrap();
    let kinds: Vec<u16> = ts.tokens.iter().map(|t| t.kind).collect();
    assert_eq!(kinds, [STRING, NUMBER]);
}

#[test]
fn e001_cases() {
    for src in ["x = 'abc\n", "/* never closed", "x = 'abc"] {
        let err = lex(&SourceText::from_normalized(src), Language::Ddls).unwrap_err();
        assert_eq!(err.code, Code::E001, "{src}");
    }
}

#[test]
fn e002_cases() {
    for src in ["define view entity X { a ", "select from t { a ) }", "a ]"] {
        let ts = lex(&SourceText::from_normalized(src), Language::Ddls).unwrap();
        assert_eq!(structure::check(&ts).unwrap_err().code, Code::E002, "{src}");
    }
}

#[test]
fn missing_semicolon_is_not_an_error() {
    let ts = lex(
        &SourceText::from_normalized("managed implementation in class zbp unique"),
        Language::Bdef,
    )
    .unwrap();
    assert!(structure::check(&ts).is_ok());
}

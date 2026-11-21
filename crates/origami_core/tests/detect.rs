use origami_core::detect::{FileKind, LanguageHint, by_file_name, detect, sniff};
use origami_core::{Code, Language};
use origami_syntax::SourceText;

#[test]
fn file_patterns() {
    let cases = [
        ("zcl_x.clas.abap", Some(Language::Abap)),
        ("zcl_x.clas.locals_imp.abap", Some(Language::Abap)),
        ("zcl_x.clas.testclasses.abap", Some(Language::Abap)),
        ("zif_x.intf.abap", Some(Language::Abap)),
        ("zfg.fugr.lzfgtop.abap", Some(Language::Abap)),
        ("#dmo#i_travel.ddls.asddls", Some(Language::Ddls)),
        ("zc_x.ddlx.asddlxs", Some(Language::Ddlx)),
        ("zx.dcls.asdcls", Some(Language::Dcl)),
        ("zx.bdef.asbdef", Some(Language::Bdef)),
        ("zx.srvd.srvdsrv", Some(Language::Srvd)),
        ("ZCL_X.CLAS.ABAP", Some(Language::Abap)),
    ];
    for (name, lang) in cases {
        assert_eq!(by_file_name(name), FileKind::Lang(lang.unwrap()), "{name}");
    }
    assert_eq!(by_file_name("x.acds"), FileKind::CdsFamily);
    assert_eq!(by_file_name("zcl_x.clas.xml"), FileKind::Unsupported);
}

fn sniffed(src: &str) -> Option<Language> {
    sniff(&SourceText::from_normalized(src))
}

#[test]
fn sniffing_keywords() {
    let cases = [
        (
            "define view entity X as select from t { a }",
            Language::Ddls,
        ),
        (
            "define root view entity X as select from t { a }",
            Language::Ddls,
        ),
        (
            "define transient view entity X provider contract analytical_query as projection on Y { a }",
            Language::Ddls,
        ),
        ("define view X as select from t { a }", Language::Ddls),
        (
            "define table entity X { key a : abap.int4; }",
            Language::Ddls,
        ),
        (
            "define table function X returns { a : abap.int4; } implemented by method c=>m;",
            Language::Ddls,
        ),
        (
            "define scalar function X returns abap.int4;",
            Language::Ddls,
        ),
        (
            "define external entity X { a : abap.int4; }",
            Language::Ddls,
        ),
        (
            "define hierarchy X as parent child hierarchy( source Y child to parent association _p start where a = 1 )",
            Language::Ddls,
        ),
        (
            "define custom entity X { key a : abap.int4; }",
            Language::Ddls,
        ),
        (
            "@EndUserText.label: 'Discount'\ndefine type /DMO/BT_Discount: abap.int1",
            Language::Ddls,
        ),
        (
            "define abstract entity X { a : abap.int4; }",
            Language::Ddls,
        ),
        ("extend view entity X with { a }", Language::Ddls),
        ("annotate entity X with { a; }", Language::Ddlx),
        ("define role X { grant select on Y; }", Language::Dcl),
        ("define accesspolicy X { }", Language::Dcl),
        (
            "define aspect X as select from Y with user element u { a }",
            Language::Dcl,
        ),
        ("define pfcg_mapping X between Y and Z { }", Language::Dcl),
        (
            "managed implementation in class zbp unique;",
            Language::Bdef,
        ),
        ("unmanaged;", Language::Bdef),
        ("projection;", Language::Bdef),
        ("interface;", Language::Bdef),
        ("abstract;", Language::Bdef),
        ("extend behavior for X { }", Language::Bdef),
        ("define service X { expose Y; }", Language::Srvd),
        ("extend service X { expose Y; }", Language::Srvd),
    ];
    for (src, lang) in cases {
        assert_eq!(sniffed(src), Some(lang), "{src}");
    }
}

#[test]
fn sniffing_skips_annotations_and_comments() {
    let src = "// comment\n@AccessControl.authorizationCheck: #NOT_REQUIRED\n@EndUserText.label: 'Travel'\n\
               @ObjectModel: { usageType: { serviceQuality: #X, sizeCategory: #S } }\n\
               @Metadata.allowExtensions: true\ndefine root view entity ZI_Travel as select from t { a }";
    assert_eq!(sniffed(src), Some(Language::Ddls));
}

#[test]
fn hints() {
    let cds = SourceText::from_normalized("annotate entity X with { a; }");
    let garbage = SourceText::from_normalized("hello world");
    let code = |r: Result<Language, origami_syntax::Diagnostic>| r.map_err(|d| d.code);
    assert_eq!(
        code(detect(&cds, &LanguageHint::Path("x.txt".into()))),
        Err(Code::E011)
    );
    assert_eq!(
        code(detect(&cds, &LanguageHint::StdinPath("x.txt".into()))),
        Ok(Language::Ddlx)
    );
    assert_eq!(
        code(detect(&cds, &LanguageHint::LanguageId("acds".into()))),
        Ok(Language::Ddlx)
    );
    assert_eq!(
        code(detect(&garbage, &LanguageHint::CdsFamily)),
        Err(Code::E010)
    );
    assert_eq!(
        code(detect(&garbage, &LanguageHint::LanguageId("abap".into()))),
        Ok(Language::Abap)
    );
    assert_eq!(
        code(detect(&garbage, &LanguageHint::Explicit(Language::Bdef))),
        Ok(Language::Bdef)
    );
}

use origami_core::{Code, LanguageHint, Options, Status, format};

fn with(width: u16, indent: u8) -> Options {
    let mut o = Options::default();
    o.lang.line_width = width;
    o.lang.indent_width = indent;
    o
}

#[test]
fn bounds() {
    assert!(with(40, 1).validate().is_ok());
    assert!(with(255, 8).validate().is_ok());
    assert_eq!(with(39, 2).validate().unwrap_err().code, Code::E020);
    assert_eq!(with(256, 2).validate().unwrap_err().code, Code::E020);
    assert_eq!(with(120, 0).validate().unwrap_err().code, Code::E020);
    assert_eq!(with(120, 9).validate().unwrap_err().code, Code::E020);
}

#[test]
fn invalid_options_fail_format() {
    let r = format(
        b"DATA x.\n",
        &LanguageHint::Path("a.prog.abap".into()),
        &with(300, 2),
    );
    assert_eq!(r.status, Status::Error);
    assert!(r.has(Code::E020));
}

#[test]
fn identity_output_and_status() {
    let src = b"REPORT z.\r\nDATA x TYPE i.\r\n";
    let r = format(
        src,
        &LanguageHint::Path("z.prog.abap".into()),
        &Options::default(),
    );
    assert_eq!(r.status, Status::Unchanged);
    assert_eq!(r.output.as_deref(), Some(&src[..]));
    assert!(!r.changed);
}

#[test]
fn structural_error_leaves_no_output() {
    let r = format(
        b"IF x = 1.\n",
        &LanguageHint::Path("z.prog.abap".into()),
        &Options::default(),
    );
    assert_eq!(r.status, Status::Error);
    assert!(r.output.is_none());
    assert!(r.has(Code::E004));
    assert_eq!(r.diagnostics[0].range.start.line, 1);
}

#[test]
fn invalid_utf8_is_e005() {
    let r = format(
        b"DATA x.\xff\n",
        &LanguageHint::Path("z.prog.abap".into()),
        &Options::default(),
    );
    assert!(r.has(Code::E005));
}

/// CDS-KEY-001: the names of non-key elements and their annotations line up
/// under the names after `key `, at every indent width and in both styles.
#[test]
fn non_key_elements_line_up_under_key_names() {
    use origami_core::{Language, Style};
    let src = "define view entity Z as select from t\n{\n  key t.id as Id,\n  @Semantics.text: true\n  t.name as Name,\n  _x\n}\n";
    let col = |out: &str, needle: &str| {
        let line = out.lines().find(|l| l.contains(needle)).expect(needle);
        line.find(needle).unwrap()
    };
    for style in [Style::Aligned, Style::Compact] {
        for indent in 1..=8 {
            let mut o = with(120, indent);
            o.lang.style = style;
            let r = format(src.as_bytes(), &LanguageHint::Explicit(Language::Ddls), &o);
            let out = String::from_utf8(r.output.unwrap()).unwrap();
            let key = col(&out, "key t.id") + 4;
            assert_eq!(
                col(&out, "t.name"),
                key,
                "{style:?} indent {indent}:\n{out}"
            );
            assert_eq!(
                col(&out, "@Semantics"),
                key,
                "{style:?} indent {indent}:\n{out}"
            );
            assert_eq!(col(&out, "_x"), key, "{style:?} indent {indent}:\n{out}");
        }
    }
}

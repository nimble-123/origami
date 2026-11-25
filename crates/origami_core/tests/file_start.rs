//! ABAP-BLANK-002 at the start of a file: a file that starts with
//! declarations (an include, a code snippet) gets a blank line after them;
//! a program that starts with `REPORT` does not.

use origami_core::{LanguageHint, Options, Style, format};

fn fmt(src: &str, style: Style) -> String {
    let mut o = Options::default();
    o.lang.style = style;
    let r = format(
        src.as_bytes(),
        &LanguageHint::Path("z.prog.abap".into()),
        &o,
    );
    String::from_utf8(r.output.expect("output")).unwrap()
}

#[test]
fn declarations_at_the_start_of_a_file() {
    let snippet = "DATA lv_a TYPE i.\nlv_a = 1.\n";
    assert_eq!(
        fmt(snippet, Style::Aligned),
        "DATA lv_a TYPE i.\n\nlv_a = 1.\n"
    );
    assert_eq!(fmt(snippet, Style::Compact), snippet);
    let program = "REPORT z.\nDATA lv_a TYPE i.\nlv_a = 1.\n";
    assert_eq!(fmt(program, Style::Aligned), program);
}

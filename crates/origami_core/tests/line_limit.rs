//! INVARIANTS 1.5: no ABAP output line exceeds 255 characters unless the same
//! input line already did. A unit that would is left as written (W002).

use origami_core::{Code, FormatResult, LanguageHint, Options, format};

/// Formats, checks rule 1.5 and idempotency, and returns the result.
fn run(src: &str) -> FormatResult {
    let hint = LanguageHint::Path("z.prog.abap".into());
    let r = format(src.as_bytes(), &hint, &Options::default());
    assert!(
        !r.has(Code::X001) && !r.has(Code::X002),
        "{:?}",
        r.diagnostics
    );
    let out = String::from_utf8(r.output.clone().expect("output")).unwrap();
    for line in out.lines() {
        assert!(
            line.chars().count() <= 255 || src.lines().any(|l| l == line),
            "new line of {} characters: {line}",
            line.chars().count()
        );
    }
    let again = format(out.as_bytes(), &hint, &Options::default());
    assert_eq!(
        again.output.as_deref(),
        Some(out.as_bytes()),
        "not idempotent"
    );
    r
}

#[test]
fn end_of_line_comment_is_left_as_written() {
    let src = format!("IF a = b.\nWRITE a. \"{}\nENDIF.\n", "x".repeat(245));
    assert!(run(&src).has(Code::W002));
}

#[test]
fn own_line_comment_that_gains_a_space_is_left_as_written() {
    // ABAP-COMMENT-003 would turn the 255-character line into 256.
    let src = format!("WRITE a.\n\"{}\n", "x".repeat(254));
    let r = run(&src);
    assert!(r.has(Code::W002), "{:?}", r.diagnostics);
    assert_eq!(r.output.as_deref(), Some(src.as_bytes()));
}

#[test]
fn own_line_comment_that_is_indented_is_left_as_written() {
    // Indenting the 254-character line by two levels makes it 257.
    let src = format!(
        "IF a = b.\nIF c = d.\n\"{}\nENDIF.\nENDIF.\n",
        "x".repeat(253)
    );
    assert!(run(&src).has(Code::W002));
}

#[test]
fn own_line_comment_in_a_file_without_statements_is_left_as_written() {
    let src = format!("\"{}\n", "x".repeat(254));
    let r = run(&src);
    assert!(r.has(Code::W002), "{:?}", r.diagnostics);
    assert_eq!(r.output.as_deref(), Some(src.as_bytes()));
}

#[test]
fn overlong_input_line_may_stay_overlong() {
    let src = format!("WRITE a.\n\"{}\n", "x".repeat(300));
    let r = format(
        src.as_bytes(),
        &LanguageHint::Path("z.prog.abap".into()),
        &Options::default(),
    );
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let out = format!("WRITE a.\n\" {}\n", "x".repeat(300));
    assert_eq!(r.output.as_deref(), Some(out.as_bytes()));
}

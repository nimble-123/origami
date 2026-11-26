//! INVARIANTS 1.5: no ABAP output line exceeds 255 characters unless the same
//! input line already did. An own-line `"` comment line that formatting would
//! push past 255 keeps its text and is indented only as far as fits
//! (ABAP-COMMENT-003); any other unit that would is left as written. Both
//! report W002.

use origami_core::{Code, FormatResult, LanguageHint, Options, Style, format};

/// Formats, checks rule 1.5 and idempotency, and returns the result.
fn run(src: &str) -> FormatResult {
    run_with(src, &Options::default())
}

fn run_with(src: &str, opts: &Options) -> FormatResult {
    let hint = LanguageHint::Path("z.prog.abap".into());
    let r = format(src.as_bytes(), &hint, opts);
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
    let again = format(out.as_bytes(), &hint, opts);
    assert_eq!(
        again.output.as_deref(),
        Some(out.as_bytes()),
        "not idempotent"
    );
    // The second run spares the same comment lines again.
    assert_eq!(
        w002_lines(&again, "Comment line"),
        w002_lines(&r, "Comment line"),
        "second run: {:?}",
        again.diagnostics
    );
    r
}

/// Lines (1-based) of the W002 diagnostics whose message starts with `what`.
fn w002_lines(r: &FormatResult, what: &str) -> Vec<u32> {
    r.diagnostics
        .iter()
        .filter(|d| d.code == Code::W002 && d.message.starts_with(what))
        .map(|d| d.range.start.line)
        .collect()
}

fn output(r: &FormatResult) -> &str {
    std::str::from_utf8(r.output.as_deref().expect("output")).unwrap()
}

#[test]
fn end_of_line_comment_is_left_as_written() {
    let src = format!("IF a = b.\nWRITE a. \"{}\nENDIF.\n", "x".repeat(245));
    let r = run(&src);
    assert!(r.has(Code::W002));
    assert!(
        w002_lines(&r, "Comment line").is_empty(),
        "{:?}",
        r.diagnostics
    );
}

#[test]
fn own_line_comment_that_would_gain_a_space_keeps_its_text() {
    // ABAP-COMMENT-003 would turn the 255-character line into 256: the
    // comment stays as written, the statement before it is formatted.
    let comment = format!("\"{}", "x".repeat(254));
    let r = run(&format!("write  a.\n{comment}\n"));
    assert_eq!(output(&r), format!("WRITE a.\n{comment}\n"));
    assert_eq!(w002_lines(&r, "Comment line"), [2], "{:?}", r.diagnostics);
    assert_eq!(r.diagnostics.len(), 1, "{:?}", r.diagnostics);
}

#[test]
fn own_line_comment_that_would_be_indented_is_indented_as_far_as_fits() {
    // Two levels of indentation and the space would make the 254-character
    // line 257: it keeps its text and moves in by one column only.
    let comment = format!("\"{}", "x".repeat(253));
    let r = run(&format!(
        "if a = b.\nif c = d.\n{comment}\nendif.\nendif.\n"
    ));
    assert_eq!(
        output(&r),
        format!("IF a = b.\n  IF c = d.\n {comment}\n  ENDIF.\nENDIF.\n")
    );
    assert_eq!(w002_lines(&r, "Comment line"), [3], "{:?}", r.diagnostics);
    assert_eq!(r.diagnostics.len(), 1, "{:?}", r.diagnostics);
}

#[test]
fn spared_comment_keeps_its_text_even_where_the_space_would_fit() {
    // Formatted, the line would be 4 + 253 = 257 characters. With the space
    // it would fit at indent 2, but a spared comment always keeps its text:
    // indent 3 is as far as the unchanged 252 characters fit.
    let comment = format!("\"{}", "x".repeat(251));
    let src = format!("if a = b.\nif c = d.\n{comment}\nwrite a.\nendif.\nendif.\n");
    let expected =
        format!("IF a = b.\n  IF c = d.\n   {comment}\n    WRITE a.\n  ENDIF.\nENDIF.\n");
    let r = run(&src);
    assert_eq!(output(&r), expected);
    assert_eq!(w002_lines(&r, "Comment line"), [3], "{:?}", r.diagnostics);
    // The compact style inserts no space; the indent alone is too much.
    let mut compact = Options::default();
    compact.lang.style = Style::Compact;
    let r = run_with(&src, &compact);
    assert_eq!(output(&r), expected);
    assert_eq!(w002_lines(&r, "Comment line"), [3], "{:?}", r.diagnostics);
}

#[test]
fn comment_that_fits_formatted_is_formatted() {
    // 4 + 251 = 255 characters with the space: nothing is spared.
    let r = run(&format!(
        "if a = b.\nif c = d.\n\"{}\nwrite a.\nendif.\nendif.\n",
        "x".repeat(249)
    ));
    assert_eq!(
        output(&r),
        format!(
            "IF a = b.\n  IF c = d.\n    \" {}\n    WRITE a.\n  ENDIF.\nENDIF.\n",
            "x".repeat(249)
        )
    );
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
}

#[test]
fn star_comment_stays_in_column_1() {
    let comment = format!("*{}", "x".repeat(254));
    let r = run(&format!("if a = b.\n{comment}\nwrite a.\nendif.\n"));
    assert_eq!(
        output(&r),
        format!("IF a = b.\n{comment}\n  WRITE a.\nENDIF.\n")
    );
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
}

#[test]
fn spared_comment_next_to_a_statement_left_as_written() {
    // The 255-character statement would be indented: it is left as written
    // (W002), the comment line is spared on its own, the rest is formatted.
    let long = format!("write '{}'.", "x".repeat(246));
    let comment = format!("\"{}", "x".repeat(254));
    let r = run(&format!("if a = b.\n{long}\nendif.\n{comment}\n"));
    assert_eq!(
        output(&r),
        format!("IF a = b.\n{long}\nENDIF.\n{comment}\n")
    );
    assert_eq!(w002_lines(&r, "Comment line"), [4], "{:?}", r.diagnostics);
    assert_eq!(w002_lines(&r, "Unit"), [2], "{:?}", r.diagnostics);
    assert_eq!(r.diagnostics.len(), 2, "{:?}", r.diagnostics);
}

#[test]
fn own_line_comment_in_a_file_without_statements_is_left_as_written() {
    let src = format!("\"{}\n", "x".repeat(254));
    let r = run(&src);
    assert_eq!(w002_lines(&r, "Comment line"), [1], "{:?}", r.diagnostics);
    assert_eq!(r.diagnostics.len(), 1, "{:?}", r.diagnostics);
    assert_eq!(r.output.as_deref(), Some(src.as_bytes()));
}

#[test]
fn comment_line_inside_a_statement_is_indented_as_far_as_fits() {
    // The comment leads a parameter at column 3: 3 + 254 would be 257.
    let comment = format!("\"{}", "x".repeat(252));
    let r = run(&format!(
        "lo_obj->do_it(\n  iv_a = 1\n{comment}\n  iv_b = 2 ).\n"
    ));
    assert_eq!(
        output(&r),
        format!("lo_obj->do_it(\n  iv_a = 1\n  {comment}\n  iv_b = 2 ).\n")
    );
    assert_eq!(w002_lines(&r, "Comment line"), [3], "{:?}", r.diagnostics);
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

/// The self-check still quarantines a broken unit next to a spared comment.
#[cfg(feature = "fault-injection")]
#[test]
fn spared_comment_next_to_a_quarantined_unit() {
    let comment = format!("\"{}", "x".repeat(253));
    let mut opts = Options::default();
    opts.fault = Some(origami_core::Fault::DropSpaceOnce);
    let r = run_with(
        &format!("if a = b.\nif c = d.\n{comment}\nendif.\nendif.\n"),
        &opts,
    );
    assert_eq!(
        output(&r),
        format!("if a = b.\n  IF c = d.\n {comment}\n  ENDIF.\nENDIF.\n")
    );
    assert!(r.has(Code::W001), "{:?}", r.diagnostics);
    assert_eq!(w002_lines(&r, "Comment line"), [3], "{:?}", r.diagnostics);
}

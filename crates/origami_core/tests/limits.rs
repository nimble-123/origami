//! Pathological input stays cheap: deep nesting is left as written beyond
//! the nesting limit (W004), nested choices are decided once, and inputs or
//! outputs over the size limits leave the file as written (W004). Each case
//! takes milliseconds; a quadratic or exponential regression takes minutes
//! or exhausts memory, so the generous time bound fails it even in debug
//! builds on slow runners.

use origami_core::{
    Code, FormatResult, Language, LanguageHint, MAX_INPUT_LEN, Options, Status, format,
};
use std::time::{Duration, Instant};

const BOUND: Duration = Duration::from_secs(10);

/// Formats with the fuzz targets' invariants (no X001/X002, formatted output
/// formats to itself) and returns the first result.
fn run(lang: Language, src: &str, opts: &Options) -> FormatResult {
    let hint = LanguageHint::Explicit(lang);
    let start = Instant::now();
    let r = format(src.as_bytes(), &hint, opts);
    assert!(
        start.elapsed() < BOUND,
        "{lang:?}: took {:?}",
        start.elapsed()
    );
    assert!(
        !r.has(Code::X001) && !r.has(Code::X002),
        "{:?}",
        r.diagnostics
    );
    let out = r.output.clone().expect("output");
    let again = format(&out, &hint, opts);
    assert_eq!(again.output.as_deref(), Some(&out[..]), "not idempotent");
    r
}

/// W004 reported once, for units nested deeper than the limit.
fn assert_depth_limited(r: &FormatResult) {
    let w004 = r
        .diagnostics
        .iter()
        .filter(|d| d.code == Code::W004)
        .count();
    assert_eq!(w004, 1, "{:?}", r.diagnostics);
    assert!(r.stats.depth_limited > 0);
}

fn nested(open: &str, inner: &str, close: &str, n: usize) -> String {
    format!("{}{inner}{}", open.repeat(n), close.repeat(n))
}

#[test]
fn deep_abap_blocks() {
    let n = 5000;
    let src = format!(
        "REPORT z.\n{}x = 1.\n{}",
        "IF a = b.\n".repeat(n),
        "ENDIF.\n".repeat(n)
    );
    let r = run(Language::Abap, &src, &Options::default());
    assert_depth_limited(&r);
    let out = String::from_utf8(r.output.unwrap()).unwrap();
    // Laid out down to the limit, as written below it.
    assert!(out.contains(&format!("\n{}IF a = b.\n", "  ".repeat(100))));
    assert!(!out.contains(&format!("\n{}IF a = b.\n", "  ".repeat(101))));
}

#[test]
fn deep_abap_brackets() {
    let src = format!("REPORT z.\nx = {}.\n", nested("( ", "1", " )", 5000));
    assert_depth_limited(&run(Language::Abap, &src, &Options::default()));
    let src = format!("REPORT z.\nx = {}.\n", nested("f( ", "1", " )", 5000));
    assert_depth_limited(&run(Language::Abap, &src, &Options::default()));
}

#[test]
fn deep_cds_braces_and_parens() {
    let anno = format!(
        "@A: {}\ndefine view entity Z as select from t {{ key a }}\n",
        nested("{ a: ", "1", " }", 5000)
    );
    assert_depth_limited(&run(Language::Ddls, &anno, &Options::default()));
    let blocks = format!(
        "define view entity Z as select from t {}\n",
        nested("{ ", "a", " }", 5000)
    );
    assert_depth_limited(&run(Language::Ddls, &blocks, &Options::default()));
    let parens = format!(
        "define view entity Z as select from t {{ key {} as x }}\n",
        nested("(", "a", ")", 5000)
    );
    assert_depth_limited(&run(Language::Ddls, &parens, &Options::default()));
}

#[test]
fn nesting_at_the_limit_is_laid_out() {
    // 99 blocks around a statement with one bracket level: 100 in all.
    let src = format!(
        "REPORT z.\n{}x = f( 1 ).\n{}",
        "IF a = b.\n".repeat(99),
        "ENDIF.\n".repeat(99)
    );
    let r = run(Language::Abap, &src, &Options::default());
    assert!(!r.has(Code::W004), "{:?}", r.diagnostics);
    assert_eq!(r.stats.depth_limited, 0);
}

/// Both layouts of a call hold its arguments: built or decided once per
/// level, not once per layout (which doubles at every level).
#[test]
fn nested_calls_are_not_exponential() {
    for args in ["a = 1 b = ", "EXPORTING a = 1 b = "] {
        let src = format!(
            "REPORT z.\nx = {}1{}.\n",
            (0..60).map(|k| format!("f{k}( {args}")).collect::<String>(),
            " )".repeat(60)
        );
        let r = run(Language::Abap, &src, &Options::default());
        assert!(!r.has(Code::W004), "{:?}", r.diagnostics);
    }
    let src = format!(
        "REPORT z.\nx = {}.\n",
        nested("VALUE #( a = 1 b = ", "1", " )", 60)
    );
    run(Language::Abap, &src, &Options::default());
}

/// Explaining the breaks looks up comments and rule spans once per break,
/// not by scanning all of them for each.
#[test]
fn explaining_many_breaks_is_not_quadratic() {
    let src = (0..40_000)
        .map(|k| format!("lv_a = {k}."))
        .collect::<Vec<_>>()
        .join(" ");
    let start = Instant::now();
    let e = origami_core::explain(
        src.as_bytes(),
        &LanguageHint::Explicit(Language::Abap),
        &Options::default(),
        &[],
    );
    assert!(start.elapsed() < BOUND, "took {:?}", start.elapsed());
    assert_eq!(e.notes.len(), 39_999, "{:?}", e.result.diagnostics);
}

#[test]
fn input_over_the_size_limit_is_left_as_written() {
    let src = "WRITE 'x'.\n".repeat(MAX_INPUT_LEN / 11 + 1);
    assert!(src.len() > MAX_INPUT_LEN);
    let r = run(Language::Abap, &src, &Options::default());
    assert_eq!(r.status, Status::Unchanged);
    assert!(r.has(Code::W004));
    assert_eq!(r.output.as_deref(), Some(src.as_bytes()));
}

#[test]
fn output_over_the_size_limit_is_left_as_written() {
    // 100 levels at indent 8 turn 6-byte lines into lines of up to 806.
    let block = format!("{}x.\n{}", "IF a.\n".repeat(100), "ENDIF.\n".repeat(100));
    let src = format!("REPORT z.\n{}", block.repeat(400));
    let mut opts = Options::default();
    opts.lang.indent_width = 8;
    let r = run(Language::Abap, &src, &opts);
    assert_eq!(r.status, Status::Unchanged);
    assert!(r.has(Code::W004), "{:?}", r.diagnostics);
    assert_eq!(r.output.as_deref(), Some(src.as_bytes()));
}

#![no_main]
//! Invariants, at the default setting and at `variant`: no X001 or X002,
//! and formatted output formats to itself.

use libfuzzer_sys::fuzz_target;
use origami_core::{Case, Code, Language, LanguageHint, Options, format};

/// A second setting derived from the input length, so fuzzing also covers
/// other line widths, indents and keyword cases without changing the input
/// format (`crates/origami_core/tests/fuzz_regressions.rs` does the same).
fn variant(n: usize) -> Options {
    let mut o = Options::default();
    o.lang.line_width = 40 + (n * 7 % 216) as u16;
    o.lang.indent_width = 1 + (n % 8) as u8;
    o.lang.abap_keyword_case = if n % 2 == 1 { Case::Lower } else { Case::Upper };
    o
}

fuzz_target!(|data: &[u8]| {
    let hint = LanguageHint::Explicit(Language::Abap);
    for opts in [Options::default(), variant(data.len())] {
        let r = format(data, &hint, &opts);
        assert!(
            !r.has(Code::X001) && !r.has(Code::X002),
            "{:?}",
            r.diagnostics
        );
        if let Some(out) = r.output {
            let again = format(&out, &hint, &opts);
            assert_eq!(again.output.as_deref(), Some(&out[..]), "not idempotent");
        }
    }
});

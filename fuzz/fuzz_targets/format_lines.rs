#![no_main]
//! Invariants of `format_lines`, at the default setting and at `variant`:
//! no X001 or X002, and a result for every source that is not an error.
//! The first byte selects the language (ABAP or a CDS-family language); the
//! line range is derived from the input length.

use libfuzzer_sys::fuzz_target;
use origami_core::{Case, Code, Language, LanguageHint, Options, Status, Style, format_lines};

const LANGS: [Language; 6] = [
    Language::Abap,
    Language::Ddls,
    Language::Ddlx,
    Language::Dcl,
    Language::Bdef,
    Language::Srvd,
];

/// A second setting derived from the input length, so fuzzing also covers
/// other line widths, indents and keyword cases without changing the input
/// format (`crates/origami_core/tests/fuzz_regressions.rs` does the same).
fn variant(n: usize) -> Options {
    let mut o = Options::default();
    o.lang.line_width = 40 + (n * 7 % 216) as u16;
    o.lang.indent_width = 1 + (n % 8) as u8;
    o.lang.abap_keyword_case = if n % 2 == 1 { Case::Lower } else { Case::Upper };
    o.lang.style = if n / 2 % 2 == 1 {
        Style::Compact
    } else {
        Style::Aligned
    };
    o
}

/// Lines `a..=b` (1-based), derived from the input length.
fn lines(n: usize) -> (u32, u32) {
    let a = 1 + (n % 13) as u32;
    (a, a + (n % 5) as u32)
}

fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let hint = LanguageHint::Explicit(LANGS[first as usize % LANGS.len()]);
    for opts in [Options::default(), variant(rest.len())] {
        let r = format_lines(rest, &hint, &opts, &[lines(rest.len())]);
        assert!(
            !r.has(Code::X001) && !r.has(Code::X002),
            "{:?}",
            r.diagnostics
        );
        assert_eq!(r.output.is_some(), r.status != Status::Error);
    }
});

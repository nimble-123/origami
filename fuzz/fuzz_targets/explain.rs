#![no_main]
//! Invariants of `explain`, at the default setting and at `variant`: no
//! X001 or X002, and the same result as `format_lines`. The first byte
//! selects the language (ABAP or a CDS-family language); the input length
//! selects the whole file or a line range.

use libfuzzer_sys::fuzz_target;
use origami_core::{Case, Code, Language, LanguageHint, Options, Style, explain, format_lines};

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

/// No range (the whole file) for an even `n`, else lines derived from the
/// input length.
fn lines(n: usize) -> Vec<(u32, u32)> {
    if n % 2 == 0 {
        return Vec::new();
    }
    let a = 1 + (n % 13) as u32;
    vec![(a, a + (n % 5) as u32)]
}

fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let hint = LanguageHint::Explicit(LANGS[first as usize % LANGS.len()]);
    let ranges = lines(rest.len());
    for opts in [Options::default(), variant(rest.len())] {
        let e = explain(rest, &hint, &opts, &ranges);
        assert!(
            !e.result.has(Code::X001) && !e.result.has(Code::X002),
            "{:?}",
            e.result.diagnostics
        );
        assert_eq!(e.result, format_lines(rest, &hint, &opts, &ranges));
    }
});

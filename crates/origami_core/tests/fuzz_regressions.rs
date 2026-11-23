//! Inputs found by `cargo fuzz` (fuzz/found/), checked with the fuzz targets'
//! invariants (at the default setting and at a second one derived from the
//! input length): no X001/X002, and formatted output formats to itself. Files
//! named `open-*` are known open findings and are skipped. `lines-*` and
//! `explain-*` files come from the `format_lines` and `explain` targets and
//! are checked with their invariants.

use origami_core::{
    Case, Code, Language, LanguageHint, Options, Status, explain, format, format_lines,
};
use std::path::Path;

/// The languages of the `format_lines` and `explain` targets.
const ALL: [Language; 6] = [
    Language::Abap,
    Language::Ddls,
    Language::Ddlx,
    Language::Dcl,
    Language::Bdef,
    Language::Srvd,
];

const CDS: [Language; 5] = [
    Language::Ddls,
    Language::Ddlx,
    Language::Dcl,
    Language::Bdef,
    Language::Srvd,
];

/// Same as `variant` in the fuzz targets.
fn variant(n: usize) -> Options {
    let mut o = Options::default();
    o.lang.line_width = 40 + (n * 7 % 216) as u16;
    o.lang.indent_width = 1 + (n % 8) as u8;
    o.lang.abap_keyword_case = if n % 2 == 1 { Case::Lower } else { Case::Upper };
    o
}

fn check(name: &str, lang: Language, data: &[u8]) {
    let hint = LanguageHint::Explicit(lang);
    for opts in [Options::default(), variant(data.len())] {
        let r = format(data, &hint, &opts);
        assert!(
            !r.has(Code::X001) && !r.has(Code::X002),
            "{name}: {:?}",
            r.diagnostics
        );
        if let Some(out) = r.output {
            let again = format(&out, &hint, &opts);
            assert_eq!(
                again.output.as_deref(),
                Some(&out[..]),
                "{name}: not idempotent"
            );
        }
    }
}

/// Same as `lines` in fuzz/fuzz_targets/format_lines.rs.
fn lines(n: usize) -> (u32, u32) {
    let a = 1 + (n % 13) as u32;
    (a, a + (n % 5) as u32)
}

/// The invariants of fuzz/fuzz_targets/format_lines.rs and explain.rs.
fn check_lines(name: &str, data: &[u8]) {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let hint = LanguageHint::Explicit(ALL[first as usize % ALL.len()]);
    let explained = if rest.len() % 2 == 0 {
        Vec::new()
    } else {
        vec![lines(rest.len())]
    };
    for opts in [Options::default(), variant(rest.len())] {
        let r = format_lines(rest, &hint, &opts, &[lines(rest.len())]);
        assert!(
            !r.has(Code::X001) && !r.has(Code::X002),
            "{name}: {:?}",
            r.diagnostics
        );
        assert_eq!(r.output.is_some(), r.status != Status::Error, "{name}");
        let e = explain(rest, &hint, &opts, &explained);
        assert_eq!(
            e.result,
            format_lines(rest, &hint, &opts, &explained),
            "{name}"
        );
    }
}

#[test]
fn fixed_fuzz_findings_stay_fixed() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/found");
    let mut n = 0;
    for e in std::fs::read_dir(dir).unwrap() {
        let path = e.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with("open-") {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        if name.starts_with("abap-") {
            check(&name, Language::Abap, &data);
        } else if name.starts_with("lines-") || name.starts_with("explain-") {
            check_lines(&name, &data);
        } else if let Some((&first, rest)) = data.split_first() {
            // Same convention as fuzz/fuzz_targets/format_cds.rs.
            check(&name, CDS[first as usize % CDS.len()], rest);
        }
        n += 1;
    }
    assert!(n >= 4, "expected the fixed findings in fuzz/found");
}

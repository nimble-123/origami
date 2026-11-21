//! Inputs found by `cargo fuzz` (fuzz/found/), checked with the fuzz targets'
//! invariants: no X001/X002, and formatted output formats to itself. Files
//! named `open-*` are known open findings and are skipped.

use origami_core::{Code, Language, LanguageHint, Options, format};
use std::path::Path;

const CDS: [Language; 5] =
    [Language::Ddls, Language::Ddlx, Language::Dcl, Language::Bdef, Language::Srvd];

fn check(name: &str, lang: Language, data: &[u8]) {
    let hint = LanguageHint::Explicit(lang);
    let opts = Options::default();
    let r = format(data, &hint, &opts);
    assert!(!r.has(Code::X001) && !r.has(Code::X002), "{name}: {:?}", r.diagnostics);
    if let Some(out) = r.output {
        let again = format(&out, &hint, &opts);
        assert_eq!(again.output.as_deref(), Some(&out[..]), "{name}: not idempotent");
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
        } else if let Some((&first, rest)) = data.split_first() {
            // Same convention as fuzz/fuzz_targets/format_cds.rs.
            check(&name, CDS[first as usize % CDS.len()], rest);
        }
        n += 1;
    }
    assert!(n >= 4, "expected the fixed findings in fuzz/found");
}

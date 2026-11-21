#![no_main]
//! Invariants: no X001 or X002, and formatted output formats to itself. The
//! first byte selects the CDS-family language.

use libfuzzer_sys::fuzz_target;
use origami_core::{Code, Language, LanguageHint, Options, format};

const LANGS: [Language; 5] =
    [Language::Ddls, Language::Ddlx, Language::Dcl, Language::Bdef, Language::Srvd];

fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else { return };
    let hint = LanguageHint::Explicit(LANGS[first as usize % LANGS.len()]);
    let opts = Options::default();
    let r = format(rest, &hint, &opts);
    assert!(!r.has(Code::X001) && !r.has(Code::X002), "{:?}", r.diagnostics);
    if let Some(out) = r.output {
        let again = format(&out, &hint, &opts);
        assert_eq!(again.output.as_deref(), Some(&out[..]), "not idempotent");
    }
});

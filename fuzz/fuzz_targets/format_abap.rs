#![no_main]
//! Invariants: no X001 or X002, and formatted output formats to itself.

use libfuzzer_sys::fuzz_target;
use origami_core::{Code, Language, LanguageHint, Options, format};

fuzz_target!(|data: &[u8]| {
    let hint = LanguageHint::Explicit(Language::Abap);
    let opts = Options::default();
    let r = format(data, &hint, &opts);
    assert!(!r.has(Code::X001) && !r.has(Code::X002), "{:?}", r.diagnostics);
    if let Some(out) = r.output {
        let again = format(&out, &hint, &opts);
        assert_eq!(again.output.as_deref(), Some(&out[..]), "not idempotent");
    }
});

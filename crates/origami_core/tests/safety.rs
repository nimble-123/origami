//! Safety paths, reachable only through fault injection:
//! `cargo test -p origami_core --features fault-injection`.
#![cfg(feature = "fault-injection")]

use origami_core::{Code, Fault, LanguageHint, Options, Status, format};

const SRC: &[u8] = b"CLASS lcl DEFINITION.\n  PUBLIC SECTION.\nENDCLASS.\n";

fn run(fault: Fault) -> origami_core::FormatResult {
    let mut opts = Options::default();
    opts.fault = Some(fault);
    format(SRC, &LanguageHint::Path("z.clas.abap".into()), &opts)
}

#[test]
fn drop_space_once_is_quarantined() {
    let r = run(Fault::DropSpaceOnce);
    assert_eq!(r.status, Status::Unchanged);
    assert!(r.has(Code::W001));
    assert_eq!(r.output.as_deref(), Some(SRC));
    assert_eq!(r.stats.quarantined, 1);
}

#[test]
fn drop_space_always_is_x001() {
    let r = run(Fault::DropSpaceAlways);
    assert_eq!(r.status, Status::Error);
    assert!(r.has(Code::X001));
    assert!(r.output.is_none());
}

/// A file of comments only has no unit to quarantine; a mismatch there is
/// still a mismatch.
#[test]
fn mismatch_without_units_is_x001() {
    for (src, path) in [
        (&b"\" hello world\n"[..], "z.prog.abap"),
        (b"// hello world\n", "z.ddls.asddls"),
    ] {
        let mut opts = Options::default();
        opts.fault = Some(Fault::DropSpaceAlways);
        let r = format(src, &LanguageHint::Path(path.into()), &opts);
        assert_eq!(r.status, Status::Error, "{path}: {:?}", r.output);
        assert!(r.has(Code::X001), "{path}: {:?}", r.diagnostics);
        assert!(r.output.is_none());
        opts.fault = Some(Fault::DropSpaceOnce);
        let r = format(src, &LanguageHint::Path(path.into()), &opts);
        assert_eq!(r.output.as_deref(), Some(src), "{path}");
    }
}

#[test]
fn panic_is_x002() {
    std::panic::set_hook(Box::new(|_| {}));
    let r = run(Fault::Panic);
    assert_eq!(r.status, Status::Error);
    assert!(r.has(Code::X002));
    assert!(r.output.is_none());
}

#[test]
fn quarantined_units_get_no_notes() {
    // `RETURN.` has no space to drop, so the fault hits the second
    // statement, which is quarantined; its statement break gets no note.
    let src = b"RETURN. lv_b = 2.\n";
    let hint = LanguageHint::Path("z.prog.abap".into());
    let plain = origami_core::explain(src, &hint, &Options::default(), &[]);
    assert_eq!(plain.notes.len(), 1, "{:#?}", plain.notes);
    let mut opts = Options::default();
    opts.fault = Some(Fault::DropSpaceOnce);
    let e = origami_core::explain(src, &hint, &opts, &[]);
    assert!(e.result.has(Code::W001));
    assert!(e.result.changed);
    assert!(e.notes.is_empty(), "{:#?}", e.notes);
}

#[test]
fn explain_panic_is_x002_without_notes() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut opts = Options::default();
    opts.fault = Some(Fault::Panic);
    let e = origami_core::explain(SRC, &LanguageHint::Path("z.clas.abap".into()), &opts, &[]);
    assert!(e.result.has(Code::X002));
    assert!(e.notes.is_empty());
}

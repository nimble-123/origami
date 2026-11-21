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

#[test]
fn panic_is_x002() {
    std::panic::set_hook(Box::new(|_| {}));
    let r = run(Fault::Panic);
    assert_eq!(r.status, Status::Error);
    assert!(r.has(Code::X002));
    assert!(r.output.is_none());
}

//! Panics outside the core.
//!
//! The core catches its own panics and reports X002 for the file. Everything
//! else (option handling, reporting, the MCP server) runs under [`catch`]:
//! a panic there ends the run with exit 70 and an X002 diagnostic in the
//! requested format, never with Rust's exit code 101.
//!
//! The panic hook stays silent inside the core, where the panic becomes a
//! diagnostic. Outside it prints one line to stderr, unless stderr carries a
//! JSON document (`--diagnostics json` or `sarif`) that the line would
//! corrupt; there the X002 entry of the document carries the message.

use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

thread_local! {
    /// True while this thread runs inside the core's panic guard.
    static IN_CORE: Cell<bool> = const { Cell::new(false) };
    /// Location of the last panic inside the core on this thread.
    static CORE_PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The last panic outside the core, `message (at file:line)`.
static PANIC: Mutex<Option<String>> = Mutex::new(None);

/// Set when stderr carries a JSON document; the hook then prints nothing.
static QUIET: AtomicBool = AtomicBool::new(false);

pub fn install_hook() {
    std::panic::set_hook(Box::new(|info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()));
        if IN_CORE.with(Cell::get) {
            CORE_PANIC.with(|p| *p.borrow_mut() = loc);
            return;
        }
        let msg = info.payload_as_str().unwrap_or("unknown panic");
        let text = match loc {
            Some(loc) => format!("{msg} (at {loc})"),
            None => msg.to_string(),
        };
        if !QUIET.load(Ordering::Relaxed) {
            eprintln!("origami: internal error: {text}");
        }
        *PANIC.lock().unwrap_or_else(PoisonError::into_inner) = Some(text);
    }));
}

/// Keeps the hook from printing (stderr carries a JSON document).
pub fn set_quiet(quiet: bool) {
    QUIET.store(quiet, Ordering::Relaxed);
}

/// Runs a call into the core, which catches its own panics.
pub fn in_core<T>(f: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            IN_CORE.with(|c| c.set(self.0));
        }
    }
    let _reset = Reset(IN_CORE.with(|c| c.replace(true)));
    f()
}

/// Where the last panic inside the core on this thread happened.
pub fn take_core_panic() -> Option<String> {
    CORE_PANIC.with(|p| p.borrow_mut().take())
}

/// Runs `f`; a panic becomes `Err` with its message and location.
pub fn catch<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    PANIC.lock().unwrap_or_else(PoisonError::into_inner).take();
    catch_unwind(AssertUnwindSafe(f)).map_err(|payload| {
        PANIC
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .unwrap_or_else(|| {
                payload
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic".into())
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catch_returns_the_message_and_location() {
        install_hook();
        set_quiet(true);
        let err = catch(|| -> i32 { panic!("boom") }).unwrap_err();
        assert!(err.starts_with("boom (at "), "{err}");
        assert!(err.contains("panic.rs:"), "{err}");
        assert_eq!(catch(|| 7), Ok(7));
    }

    #[test]
    fn in_core_restores_the_flag() {
        assert!(!IN_CORE.with(Cell::get));
        in_core(|| assert!(IN_CORE.with(Cell::get)));
        assert!(!IN_CORE.with(Cell::get));
    }
}

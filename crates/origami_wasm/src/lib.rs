//! origami for the browser: a small C ABI over `origami_core::explain`, used
//! by the website playground (`site/`). No wasm-bindgen; JavaScript calls:
//!
//! 1. `alloc(len)` and writes the UTF-8 source into that buffer;
//! 2. `review(ptr, len, cds, width, indent, lower, compact)`; the source buffer is
//!    freed by this call;
//! 3. reads `result_len()` bytes at the returned pointer: a JSON document
//!    (see [`review_json`]); then `dealloc(result, result_len())`.
//!
//! The core does no I/O, so nothing else is needed. A panic aborts the
//! instance (wasm32 has no unwinding); the page then starts a new one.

use origami_core::{Case, LanguageHint, Options, Status, Style, explain};
use serde_json::json;
use std::cell::Cell;

thread_local! {
    static RESULT_LEN: Cell<usize> = const { Cell::new(0) };
}

/// Formats `source` and returns the result with its line-break notes as JSON:
/// `{status, changed, output, language, diagnostics: [{code, severity,
/// message, line, column}], notes: [{first, last, rule, text}]}`.
pub fn review_json(
    source: &str,
    cds: bool,
    width: u16,
    indent: u8,
    lower: bool,
    compact: bool,
) -> String {
    let hint = if cds {
        LanguageHint::CdsFamily
    } else {
        LanguageHint::StdinPath("playground.prog.abap".into())
    };
    let mut opts = Options::default();
    opts.lang.line_width = width;
    opts.lang.indent_width = indent;
    opts.lang.abap_keyword_case = if lower { Case::Lower } else { Case::Upper };
    opts.lang.style = if compact {
        Style::Compact
    } else {
        Style::Aligned
    };
    let e = explain(source.as_bytes(), &hint, &opts, &[]);
    let status = match e.result.status {
        Status::Formatted => "formatted",
        Status::Unchanged => "unchanged",
        Status::Error => "error",
        Status::Skipped => "skipped",
    };
    let diagnostics: Vec<_> = e
        .result
        .diagnostics
        .iter()
        .map(|d| {
            json!({
                "code": d.code.as_str(),
                "severity": format!("{:?}", d.severity).to_lowercase(),
                "message": d.message,
                "line": d.range.start.line,
                "column": d.range.start.column,
            })
        })
        .collect();
    let notes: Vec<_> = e
        .notes
        .iter()
        .map(|n| {
            json!({
                "first": n.lines.start(),
                "last": n.lines.end(),
                "rule": n.rule,
                "text": n.text,
            })
        })
        .collect();
    json!({
        "status": status,
        "changed": e.result.changed,
        "output": e.result.output.as_deref().map(|o| String::from_utf8_lossy(o).into_owned()),
        "language": e.result.language.map(|l| l.as_str()),
        "diagnostics": diagnostics,
        "notes": notes,
    })
    .to_string()
}

/// The size of the allocation behind a buffer of `len` bytes: at least one
/// byte, so that every buffer has a real allocation.
fn alloc_len(len: usize) -> usize {
    len.max(1)
}

/// A zeroed buffer of `len` bytes for JavaScript to fill. Buffers are boxed
/// slices, so the allocation is exactly `len.max(1)` bytes and [`dealloc`]
/// can rebuild it from the pointer and length alone.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; alloc_len(len)].into_boxed_slice()).cast::<u8>()
}

/// Takes ownership of a buffer from [`alloc`] or [`review`] again.
///
/// # Safety
/// `ptr` and `len` must come from `alloc(len)` or from `review` /
/// `result_len`, and the buffer must not be used afterwards.
unsafe fn take_buffer(ptr: *mut u8, len: usize) -> Box<[u8]> {
    // SAFETY: the caller passes a pointer and length from `alloc` or
    // `review`; both leak a `Box<[u8]>` of exactly `alloc_len(len)` bytes.
    unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, alloc_len(len))) }
}

/// Frees a buffer from [`alloc`] or [`review`].
///
/// # Safety
/// `ptr` and `len` must come from `alloc(len)` or from `review` /
/// `result_len`, and the buffer must not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    // SAFETY: forwarded from the caller.
    drop(unsafe { take_buffer(ptr, len) });
}

/// Length of the last result of [`review`].
#[unsafe(no_mangle)]
pub extern "C" fn result_len() -> usize {
    RESULT_LEN.with(Cell::get)
}

/// Formats the `len` bytes at `ptr` (frees them) and returns a pointer to the
/// JSON result; its length is `result_len()`.
///
/// # Safety
/// `ptr` must come from `alloc(len)` and hold `len` initialized bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn review(
    ptr: *mut u8,
    len: usize,
    cds: u32,
    width: u32,
    indent: u32,
    lower: u32,
    compact: u32,
) -> *mut u8 {
    // SAFETY: `ptr` comes from `alloc(len)` and JavaScript wrote `len` bytes.
    let input = unsafe { take_buffer(ptr, len) };
    let source = String::from_utf8_lossy(&input[..len]);
    let width = u16::try_from(width).unwrap_or(120);
    let indent = u8::try_from(indent).unwrap_or(2);
    let mut out =
        review_json(&source, cds != 0, width, indent, lower != 0, compact != 0).into_bytes();
    if out.is_empty() {
        // Not reachable (the JSON is never empty); keeps `alloc_len` honest.
        out.push(b' ');
    }
    // A boxed slice has capacity == length, which `dealloc(ptr, result_len())`
    // relies on.
    let out = out.into_boxed_slice();
    RESULT_LEN.with(|l| l.set(out.len()));
    Box::into_raw(out).cast::<u8>()
}

#[cfg(test)]
mod tests {
    use super::{alloc, dealloc, result_len, review, review_json};

    #[test]
    fn formats_and_explains() {
        let v: serde_json::Value = serde_json::from_str(&review_json(
            "lv_a = 1. lv_b = 2.\n",
            false,
            120,
            2,
            false,
            false,
        ))
        .unwrap();
        assert_eq!(v["status"], "formatted");
        assert_eq!(v["output"], "lv_a = 1.\nlv_b = 2.\n");
        assert_eq!(v["notes"][0]["rule"], "ABAP-STMT-001");
    }

    #[test]
    fn reports_errors() {
        let v: serde_json::Value =
            serde_json::from_str(&review_json("lv_a = 1\n", false, 120, 2, false, false)).unwrap();
        assert_eq!(v["status"], "error");
        assert_eq!(v["diagnostics"][0]["code"], "E003");
        assert!(v["output"].is_null());
    }

    /// Runs the FFI the way the page does: alloc, write, review, read, dealloc.
    fn ffi_review(source: &[u8], cds: u32) -> serde_json::Value {
        let ptr = alloc(source.len());
        // SAFETY: `alloc(len)` returns at least `len` writable bytes.
        unsafe { std::ptr::copy_nonoverlapping(source.as_ptr(), ptr, source.len()) };
        // SAFETY: `ptr` comes from `alloc(source.len())` and holds those bytes.
        let res = unsafe { review(ptr, source.len(), cds, 120, 2, 0, 0) };
        let len = result_len();
        // SAFETY: `review` returns `result_len()` initialized bytes at `res`.
        let json = unsafe { std::slice::from_raw_parts(res, len) }.to_vec();
        // SAFETY: `res` and `len` come from `review` / `result_len`.
        unsafe { dealloc(res, len) };
        serde_json::from_slice(&json).unwrap()
    }

    #[test]
    fn ffi_round_trip() {
        let v = ffi_review(b"lv_a = 1. lv_b = 2.\n", 0);
        assert_eq!(v["status"], "formatted");
        assert_eq!(v["output"], "lv_a = 1.\nlv_b = 2.\n");
        let v = ffi_review(
            b"@EndUserText.label: 'x'\ndefine view entity Z as select from t { key a }\n",
            1,
        );
        assert_eq!(v["language"], "ddls");
    }

    #[test]
    fn ffi_empty_and_invalid_input() {
        // Zero bytes still get a real allocation; invalid UTF-8 is replaced.
        assert_eq!(ffi_review(b"", 0)["status"], "unchanged");
        let _ = ffi_review(b"lv_a = '\xff\xfe'.\n", 0);
    }

    #[test]
    fn alloc_dealloc_match_for_many_sizes() {
        for len in [0, 1, 2, 7, 8, 63, 64, 65, 1000, 4096, 100_000] {
            let p = alloc(len);
            // SAFETY: the buffer has at least `len` bytes; freed with the same `len`.
            unsafe {
                std::ptr::write_bytes(p, 0xAB, len);
                dealloc(p, len);
            }
        }
    }
}

//! origami for the browser: a small C ABI over `origami_core::explain`, used
//! by the website playground (`site/`). No wasm-bindgen; JavaScript calls:
//!
//! 1. `alloc(len)` and writes the UTF-8 source into that buffer;
//! 2. `review(ptr, len, cds, width, indent, lower)`; the source buffer is
//!    freed by this call;
//! 3. reads `result_len()` bytes at the returned pointer: a JSON document
//!    (see [`review_json`]); then `dealloc(result, result_len())`.
//!
//! The core does no I/O, so nothing else is needed. A panic aborts the
//! instance (wasm32 has no unwinding); the page then starts a new one.

use origami_core::{Case, LanguageHint, Options, Status, explain};
use serde_json::json;
use std::cell::Cell;

thread_local! {
    static RESULT_LEN: Cell<usize> = const { Cell::new(0) };
}

/// Formats `source` and returns the result with its line-break notes as JSON:
/// `{status, changed, output, language, diagnostics: [{code, severity,
/// message, line, column}], notes: [{first, last, rule, text}]}`.
pub fn review_json(source: &str, cds: bool, width: u16, indent: u8, lower: bool) -> String {
    let hint = if cds {
        LanguageHint::CdsFamily
    } else {
        LanguageHint::StdinPath("playground.prog.abap".into())
    };
    let mut opts = Options::default();
    opts.lang.line_width = width;
    opts.lang.indent_width = indent;
    opts.lang.abap_keyword_case = if lower { Case::Lower } else { Case::Upper };
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

/// A buffer of `len` bytes for JavaScript to fill.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Frees a buffer from [`alloc`] or [`review`].
///
/// # Safety
/// `ptr` and `len` must come from `alloc(len)` or from `review` /
/// `result_len`, and the buffer must not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    // SAFETY: the caller passes a pointer and capacity from `alloc` or
    // `review`, which both leak a Vec<u8> of exactly that capacity.
    drop(unsafe { Vec::from_raw_parts(ptr, 0, len.max(1)) });
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
) -> *mut u8 {
    // SAFETY: `ptr` comes from `alloc(len)` and JavaScript wrote `len` bytes.
    let input = unsafe { Vec::from_raw_parts(ptr, len, len.max(1)) };
    let source = String::from_utf8_lossy(&input);
    let width = u16::try_from(width).unwrap_or(120);
    let indent = u8::try_from(indent).unwrap_or(2);
    let out = review_json(&source, cds != 0, width, indent, lower != 0).into_bytes();
    let mut out = std::mem::ManuallyDrop::new(out);
    out.shrink_to_fit();
    // `dealloc(ptr, result_len())` frees it: shrink_to_fit makes capacity
    // equal to the length for u8 buffers.
    RESULT_LEN.with(|l| l.set(out.capacity()));
    out.as_mut_ptr()
}

#[cfg(test)]
mod tests {
    use super::review_json;

    #[test]
    fn formats_and_explains() {
        let v: serde_json::Value =
            serde_json::from_str(&review_json("lv_a = 1. lv_b = 2.\n", false, 120, 2, false))
                .unwrap();
        assert_eq!(v["status"], "formatted");
        assert_eq!(v["output"], "lv_a = 1.\nlv_b = 2.\n");
        assert_eq!(v["notes"][0]["rule"], "ABAP-STMT-001");
    }

    #[test]
    fn reports_errors() {
        let v: serde_json::Value =
            serde_json::from_str(&review_json("lv_a = 1\n", false, 120, 2, false)).unwrap();
        assert_eq!(v["status"], "error");
        assert_eq!(v["diagnostics"][0]["code"], "E003");
        assert!(v["output"].is_null());
    }
}

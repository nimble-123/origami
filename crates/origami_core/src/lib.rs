//! origami core: the single `format` entry point used by the CLI, the fuzzer
//! and the benchmarks. The core performs no file, network or environment
//! access and starts no threads.

mod check;
pub mod detect;
pub mod options;
pub mod result;

pub use detect::LanguageHint;
#[cfg(feature = "fault-injection")]
pub use options::Fault;
pub use options::Options;
pub use origami_syntax::{Case, Code, Edition, Language, Position, Severity, Stats};
pub use result::{FormatResult, Reported, ReportedRange, Status};

use origami_syntax::{Diagnostic, LineIndex, SourceText};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Languages whose printer still runs in identity mode (output equals input).
/// None since all six languages format (the CDS family structurally).
pub fn identity_mode(_lang: Language) -> bool {
    false
}

/// Formats one source. Never panics: a panic inside becomes X002.
pub fn format(input: &[u8], hint: &LanguageHint, opts: &Options) -> FormatResult {
    match catch_unwind(AssertUnwindSafe(|| format_inner(input, hint, opts))) {
        Ok(r) => r,
        Err(payload) => {
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".into());
            let d = Diagnostic::new(Code::X002, format!("Internal error: {msg}"), 0..0);
            error_result(None, &d, "")
        }
    }
}

fn format_inner(input: &[u8], hint: &LanguageHint, opts: &Options) -> FormatResult {
    if let Err(d) = opts.validate() {
        return error_result(None, &d, "");
    }
    let src = match SourceText::decode(input) {
        Ok(s) => s,
        Err(d) => return error_result(None, &d, ""),
    };
    let lang = match detect::detect(&src, hint) {
        Ok(l) => l,
        Err(d) => return error_result(None, &d, src.text()),
    };
    let checked = check::run(&src, lang, opts, identity_mode(lang));
    let r = checked.result;
    let index = LineIndex::new(src.text());
    let diagnostics: Vec<Reported> = r
        .diagnostics
        .iter()
        .map(|d| Reported::from(d, &index))
        .collect();
    let failed = r
        .diagnostics
        .iter()
        .any(|d| d.severity() >= Severity::Error);
    if failed {
        return FormatResult {
            output: None,
            changed: false,
            language: Some(lang),
            status: Status::Error,
            diagnostics,
            stats: r.stats,
        };
    }
    let bytes = src.encode(&r.output);
    let changed = bytes != input;
    FormatResult {
        output: Some(bytes),
        changed,
        language: Some(lang),
        status: if changed {
            Status::Formatted
        } else {
            Status::Unchanged
        },
        diagnostics,
        stats: r.stats,
    }
}

fn error_result(lang: Option<Language>, d: &Diagnostic, text: &str) -> FormatResult {
    FormatResult {
        output: None,
        changed: false,
        language: lang,
        status: Status::Error,
        diagnostics: vec![Reported::from(d, &LineIndex::new(text))],
        stats: Stats::default(),
    }
}

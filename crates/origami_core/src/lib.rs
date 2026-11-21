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
    format_lines(input, hint, opts, &[])
}

/// Formats only the changes that touch the given input lines (1-based,
/// inclusive ranges; empty = the whole source). The source is formatted as a
/// whole; of the resulting changes, only those that touch a range are kept,
/// and the result passes the same self-check. If it cannot, the source stays
/// as written and W001 is reported.
pub fn format_lines(
    input: &[u8],
    hint: &LanguageHint,
    opts: &Options,
    lines: &[(u32, u32)],
) -> FormatResult {
    match catch_unwind(AssertUnwindSafe(|| format_inner(input, hint, opts, lines))) {
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

fn format_inner(
    input: &[u8],
    hint: &LanguageHint,
    opts: &Options,
    lines: &[(u32, u32)],
) -> FormatResult {
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
    let mut r = checked.result;
    let failed_early = r
        .diagnostics
        .iter()
        .any(|d| d.severity() >= Severity::Error);
    if !lines.is_empty() && !failed_early {
        match check::select_lines(&src, lang, &r, lines) {
            Some(mixed) => r.output = mixed,
            None => {
                r.output = src.text().to_string();
                r.diagnostics.push(Diagnostic::new(
                    Code::W001,
                    "The changes in the requested lines cannot be applied on their own; left as written",
                    0..0,
                ));
            }
        }
    }
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

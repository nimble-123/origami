//! origami core: the single `format` entry point used by the CLI, the fuzzer
//! and the benchmarks. The core performs no file, network or environment
//! access and starts no threads.

mod check;
pub mod detect;
mod explain;
pub mod options;
pub mod result;

pub use detect::LanguageHint;
pub use explain::{Explained, RULE_PHRASES};
#[cfg(feature = "fault-injection")]
pub use options::Fault;
pub use options::Options;
pub use origami_syntax::{
    BreakCause, Case, Code, Edition, Language, Note, Position, RuleId, Scope, Severity, Stats,
};
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
    guarded(|| format_inner(input, hint, opts, lines, false).result)
        .unwrap_or_else(|d| error_result(None, &d, ""))
}

/// Runs `f`; a panic inside becomes an X002 diagnostic.
fn guarded<T>(f: impl FnOnce() -> T) -> Result<T, Diagnostic> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|payload| {
        let msg = payload
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".into());
        Diagnostic::new(Code::X002, format!("Internal error: {msg}"), 0..0)
    })
}

/// A formatting result and, when explanations were requested and the
/// source formatted without error, what the explain pass needs.
pub(crate) struct Inner {
    pub result: FormatResult,
    pub parts: Option<explain::Parts>,
}

fn format_inner(
    input: &[u8],
    hint: &LanguageHint,
    opts: &Options,
    lines: &[(u32, u32)],
    explain: bool,
) -> Inner {
    let plain = |result| Inner {
        result,
        parts: None,
    };
    if let Err(d) = opts.validate() {
        return plain(error_result(None, &d, ""));
    }
    let src = match SourceText::decode(input) {
        Ok(s) => s,
        Err(d) => return plain(error_result(None, &d, "")),
    };
    let lang = match detect::detect(&src, hint) {
        Ok(l) => l,
        Err(d) => return plain(error_result(None, &d, src.text())),
    };
    let identity = identity_mode(lang);
    let checked = check::run(&src, lang, opts, identity, explain);
    // The final pass before line selection, for the explain pass.
    let full = explain.then(|| checked.result.clone());
    let check::Checked {
        result: mut r,
        quarantine,
        opaque,
        whole_file_fallback,
    } = checked;
    let failed_early = r
        .diagnostics
        .iter()
        .any(|d| d.severity() >= Severity::Error);
    let mut map = None;
    if !lines.is_empty() && !failed_early {
        match check::select_lines(&src, lang, &r, lines) {
            Some((mixed, m)) => {
                r.output = mixed;
                map = Some(m);
            }
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
        return plain(FormatResult {
            output: None,
            changed: false,
            language: Some(lang),
            status: Status::Error,
            diagnostics,
            stats: r.stats,
        });
    }
    let bytes = src.encode(&r.output);
    let changed = bytes != input;
    let result = FormatResult {
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
    };
    let parts = full.map(|full| explain::Parts {
        lang,
        identity,
        checked: check::Checked {
            result: full,
            quarantine,
            opaque,
            whole_file_fallback,
        },
        selection: (!lines.is_empty()).then_some(map),
        output_text: r.output,
        src,
    });
    Inner { result, parts }
}

/// Formats like [`format_lines`] and explains the line breaks origami
/// introduced (one [`Note`] per broken construct and cause). `result` is the
/// same as `format_lines` returns. Never panics: a panic inside becomes X002
/// with no notes.
pub fn explain(
    input: &[u8],
    hint: &LanguageHint,
    opts: &Options,
    lines: &[(u32, u32)],
) -> Explained {
    guarded(|| {
        let inner = format_inner(input, hint, opts, lines, true);
        explain::explain(inner, opts)
    })
    .unwrap_or_else(|d| Explained {
        result: error_result(None, &d, ""),
        notes: Vec::new(),
        input_text: String::new(),
        output_text: String::new(),
    })
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

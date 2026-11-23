//! `idempotency`: formats every corpus file at every combination of line
//! width, indent width and keyword case, and checks:
//! * formatting the output again gives the same output;
//! * `explain` gives the same result as `format`;
//! * an already formatted file stays unchanged when only some of its lines
//!   are formatted (`--lines`).
//!
//! The corpus run checks idempotency at the default settings only, and the
//! hash run at one other setting; widths near a construct's length and
//! unusual indents are where a second pass can choose a different layout.

use crate::corpus::{files, manifest};
use origami_core::{Case, LanguageHint, Options, explain, format, format_lines};
use std::path::PathBuf;

const WIDTHS: [u16; 8] = [40, 55, 72, 80, 100, 120, 160, 255];
const INDENTS: [u8; 5] = [1, 2, 3, 4, 8];
const CASES: [Case; 2] = [Case::Upper, Case::Lower];

pub fn options() -> Vec<Options> {
    let mut all = Vec::new();
    for &w in &WIDTHS {
        for &i in &INDENTS {
            for &c in &CASES {
                let mut o = Options::default();
                o.lang.line_width = w;
                o.lang.indent_width = i;
                o.lang.abap_keyword_case = c;
                all.push(o);
            }
        }
    }
    all
}

pub fn label(o: &Options) -> String {
    format!(
        "w{} i{} {:?}",
        o.lang.line_width, o.lang.indent_width, o.lang.abap_keyword_case
    )
}

/// All problems for one file.
fn check(name: &str, bytes: &[u8], all: &[Options]) -> Vec<String> {
    let hint = LanguageHint::Path(name.to_string());
    let mut problems = Vec::new();
    for o in all {
        let r = format(bytes, &hint, o);
        let Some(out) = &r.output else { continue };
        if format(out, &hint, o).output.as_deref() != Some(&out[..]) {
            problems.push(format!("{name} [{}]: not idempotent", label(o)));
        }
        if o.lang.indent_width == 2 && explain(bytes, &hint, o, &[]).result != r {
            problems.push(format!(
                "{name} [{}]: explain differs from format",
                label(o)
            ));
        }
    }
    // Partial formatting of a formatted file changes nothing.
    let o = Options::default();
    if let Some(out) = format(bytes, &hint, &o).output {
        let n = out.iter().filter(|&&b| b == b'\n').count() as u32 + 1;
        for range in [
            (1, 1),
            (n / 2, n / 2),
            (1, n / 2),
            (n / 3, n),
            (2, n.max(2)),
        ] {
            let r = format_lines(&out, &hint, &o, &[range]);
            if r.output.as_deref().is_some_and(|p| p != &out[..]) {
                problems.push(format!(
                    "{name} [lines {}:{}]: partial formatting changes a formatted file",
                    range.0, range.1
                ));
            }
        }
    }
    problems
}

pub fn run() -> Result<(), String> {
    let m = manifest()?;
    let mut inputs: Vec<(String, PathBuf)> = Vec::new();
    for r in &m.repo {
        inputs.extend(files(r));
    }
    if inputs.is_empty() {
        return Err("no corpus files — run `cargo xtask corpus fetch`".into());
    }
    let all = options();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = inputs.len().div_ceil(threads);
    let problems: Vec<String> = std::thread::scope(|s| {
        let handles: Vec<_> = inputs
            .chunks(chunk)
            .map(|part| {
                let all = &all;
                s.spawn(move || {
                    let mut out = Vec::new();
                    for (_, path) in part {
                        let name = path.to_string_lossy().into_owned();
                        match std::fs::read(path) {
                            Ok(bytes) => out.extend(check(&name, &bytes, all)),
                            Err(e) => out.push(format!("{name}: {e}")),
                        }
                    }
                    out
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_else(|_| vec!["panic".into()]))
            .collect()
    });
    for p in problems.iter().take(40) {
        eprintln!("FAIL {p}");
    }
    println!(
        "idempotency: {} files x {} settings: {} problems",
        inputs.len(),
        all.len(),
        problems.len()
    );
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!("{} idempotency problem(s)", problems.len()))
    }
}

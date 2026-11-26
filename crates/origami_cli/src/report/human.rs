//! Human output on stderr: one block per file with something to report, a
//! code frame for structural errors, a summary line and the next step.
//!
//! Colors when stderr is a terminal; `NO_COLOR` turns them off,
//! `CLICOLOR_FORCE=1` turns them on. The run time is shown only on a
//! terminal, so CI logs stay comparable.

use super::{FileReport, Run};
use origami_core::{Code, Reported, Severity, Status};
use std::io::{IsTerminal, Write};

/// W003 lines shown per file; the rest is counted.
const W003_SHOWN: usize = 3;
/// Changed files listed; the rest is counted.
const FILES_SHOWN: usize = 50;

struct Style {
    on: bool,
}

impl Style {
    fn paint(&self, sgr: &str, s: &str) -> String {
        if self.on {
            format!("\x1b[{sgr}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }
    fn red(&self, s: &str) -> String {
        self.paint("31", s)
    }
    fn green(&self, s: &str) -> String {
        self.paint("32", s)
    }
    fn yellow(&self, s: &str) -> String {
        self.paint("33", s)
    }
    fn cyan(&self, s: &str) -> String {
        self.paint("36", s)
    }
    fn dim(&self, s: &str) -> String {
        self.paint("2", s)
    }
    fn bold(&self, s: &str) -> String {
        self.paint("1", s)
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The source line of `d` with a marker under the reported columns.
fn code_frame(st: &Style, source: &str, d: &Reported) -> Option<String> {
    let line_no = d.range.start.line as usize;
    let line = source.lines().nth(line_no.checked_sub(1)?)?;
    let col = (d.range.start.column as usize).max(1);
    let width = if d.range.end.line == d.range.start.line {
        (d.range.end.column as usize).saturating_sub(col).max(1)
    } else {
        1
    };
    // Keep tabs so the marker lines up under the (escaped) text.
    let prefix: String = line.chars().take(col - 1).collect();
    let pad: String = super::escape_controls(&prefix)
        .chars()
        .map(|c| if c == '\t' { '\t' } else { ' ' })
        .collect();
    let gutter = format!("{line_no:>6}");
    let blank = " ".repeat(gutter.len());
    Some(format!(
        "{} {} {}\n{} {} {}{}\n",
        st.dim(&gutter),
        st.dim("│"),
        super::escape_controls(line.trim_end()),
        blank,
        st.dim("│"),
        pad,
        st.red(&"^".repeat(width.min(80)))
    ))
}

fn location(d: &Reported) -> String {
    format!("{}:{}", d.range.start.line, d.range.start.column)
}

fn write_file(
    st: &Style,
    run: &Run,
    f: &FileReport,
    diags: &[Reported],
    out: &mut dyn Write,
) -> std::io::Result<()> {
    let errors: Vec<&Reported> = diags
        .iter()
        .filter(|d| d.severity >= Severity::Error)
        .collect();
    let others: Vec<&Reported> = diags
        .iter()
        .filter(|d| d.severity < Severity::Error && d.code != Code::W003)
        .collect();
    let w003: Vec<&Reported> = diags.iter().filter(|d| d.code == Code::W003).collect();
    let path = st.bold(&super::escape_controls(&f.path));
    if f.status == Status::Error {
        let internal = errors.iter().any(|d| d.severity == Severity::Internal);
        let label = if internal {
            "internal error, left unchanged"
        } else {
            "not formatted, left unchanged"
        };
        writeln!(out, "{} {path}  {}", st.red("✗"), st.red(label))?;
    } else if f.changed {
        let (sym, label) = if run.check {
            (st.yellow("✎"), st.yellow("would change"))
        } else {
            (st.green("✎"), st.green("formatted"))
        };
        writeln!(out, "{sym} {path}  {label}")?;
    } else {
        let n = others.len() + w003.len();
        writeln!(
            out,
            "{} {path}  {}",
            st.yellow("!"),
            st.yellow(&plural(n, "warning", "warnings"))
        )?;
    }
    for d in &errors {
        let code = st.red(d.code.as_str());
        writeln!(out, "    {code} {}  {}", st.dim(&location(d)), d.message)?;
        if let Some(frame) = f.source.as_deref().and_then(|s| code_frame(st, s, d)) {
            write!(out, "{frame}")?;
        }
    }
    for d in &others {
        let code = st.yellow(d.code.as_str());
        writeln!(out, "    {code} {}  {}", st.dim(&location(d)), d.message)?;
    }
    for d in w003.iter().take(W003_SHOWN) {
        let code = st.cyan(d.code.as_str());
        writeln!(
            out,
            "    {code} {}  statement kept with its own line breaks",
            st.dim(&location(d))
        )?;
    }
    if w003.len() > W003_SHOWN {
        writeln!(
            out,
            "    {}",
            st.dim(&format!(
                "… {} more",
                plural(w003.len() - W003_SHOWN, "statement", "statements")
            ))
        )?;
    }
    Ok(())
}

pub fn write(run: &Run, out: &mut dyn Write) -> std::io::Result<()> {
    let st = Style {
        on: super::color_enabled(super::Stream::Stderr),
    };
    let (files, truncated) = run.capped();
    let mut listed_changes = 0;
    let mut hidden_changes = 0;
    for (f, diags) in &files {
        let only_change = f.changed && diags.is_empty() && f.status != Status::Error;
        let say_change = f.changed && (run.check || run.files.len() > 1);
        if diags.is_empty() && !say_change {
            continue;
        }
        if only_change {
            if listed_changes >= FILES_SHOWN {
                hidden_changes += 1;
                continue;
            }
            listed_changes += 1;
        }
        write_file(&st, run, f, diags, out)?;
    }
    if hidden_changes > 0 {
        writeln!(
            out,
            "{}",
            st.dim(&format!(
                "… and {} more",
                plural(hidden_changes, "file", "files")
            ))
        )?;
    }
    if truncated {
        writeln!(
            out,
            "{}",
            st.dim("… more diagnostics were cut by --max-diagnostics")
        )?;
    }

    // Summary.
    let sources: Vec<&FileReport> = run
        .files
        .iter()
        .filter(|f| f.status != Status::Skipped)
        .collect();
    let changed = sources.iter().filter(|f| f.changed).count();
    let errors = sources.iter().filter(|f| f.status == Status::Error).count();
    let unchanged = sources.len() - changed - errors;
    let warnings: usize = run
        .files
        .iter()
        .flat_map(|f| &f.diagnostics)
        .filter(|d| d.severity == Severity::Warning)
        .count();
    let anything = run.files.iter().any(|f| !f.diagnostics.is_empty());
    if sources.len() <= 1 && !run.check && !anything {
        return Ok(());
    }
    let mut parts = Vec::new();
    if unchanged > 0 {
        parts.push(st.green(&format!("✓ {unchanged} unchanged")));
    }
    if changed > 0 {
        let verb = if run.check {
            "would change"
        } else {
            "formatted"
        };
        let s = format!("✎ {changed} {verb}");
        parts.push(if run.check {
            st.yellow(&s)
        } else {
            st.green(&s)
        });
    }
    if errors > 0 {
        parts.push(st.red(&format!("✗ {}", plural(errors, "error", "errors"))));
    }
    if warnings > 0 {
        parts.push(st.yellow(&format!("! {}", plural(warnings, "warning", "warnings"))));
    }
    let mut tail = plural(sources.len(), "file", "files");
    if std::io::stderr().is_terminal()
        && let Some(start) = crate::run::START.get()
    {
        let ms = start.elapsed().as_millis();
        tail = format!("{tail} in {ms} ms");
    }
    writeln!(out)?;
    writeln!(
        out,
        "{}  {}",
        parts.join(&st.dim(" · ")),
        st.dim(&format!("({tail})"))
    )?;
    let hint = if errors > 0 {
        Some("files with errors were left unchanged; fix them and run origami again")
    } else if run.check && changed > 0 {
        Some("run `origami` to format, or `origami --review` to see the changes")
    } else {
        None
    };
    if let Some(h) = hint {
        writeln!(out, "{}", st.dim(&format!("→ {h}")))?;
    }
    Ok(())
}

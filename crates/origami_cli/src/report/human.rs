use super::Run;
use origami_core::{Severity, Status};
use std::io::Write;

pub fn write(run: &Run, out: &mut dyn Write) -> std::io::Result<()> {
    let (files, truncated) = run.capped();
    for (f, diags) in &files {
        for d in diags.iter() {
            let sev = match d.severity {
                Severity::Info => "info",
                Severity::Warning => "warning",
                Severity::Error => "error",
                Severity::Internal => "internal error",
            };
            writeln!(
                out,
                "{}:{}:{}: {sev}[{}]: {}",
                f.path,
                d.range.start.line,
                d.range.start.column,
                d.code.as_str(),
                d.message
            )?;
        }
        if run.check && f.changed {
            writeln!(out, "{}: would reformat", f.path)?;
        }
    }
    if truncated {
        writeln!(
            out,
            "origami: more diagnostics were cut by --max-diagnostics"
        )?;
    }
    let changed = run.files.iter().filter(|f| f.changed).count();
    let errors = run
        .files
        .iter()
        .filter(|f| f.status == Status::Error)
        .count();
    if run.files.len() > 1 || run.check {
        let verb = if run.check { "would change" } else { "changed" };
        writeln!(
            out,
            "origami: {} file(s) processed, {changed} {verb}, {errors} with errors",
            run.files.len()
        )?;
    }
    Ok(())
}

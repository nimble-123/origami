//! Output for AI agents on stderr (`--diagnostics agent`): few tokens, one
//! finding per line with a fixed prefix and a fix hint, and a final `NEXT:`
//! line that says what to do. No colors, no decoration, sorted by path.
//!
//! ```text
//! origami: 3 files, 1 would change, 1 with errors, 0 warnings
//! ERROR src/zcl_b.clas.abap:42:7 E003 Statement is not terminated by a period. Fix: end the statement with a period. The file was not changed.
//! CHANGE src/zcl_a.clas.abap
//! NOTE 4 statements without grammar keep their own line breaks (W003); no action needed.
//! NEXT: fix the errors above (those files were not changed), then run origami again.
//! ```

use super::{FileReport, Run};
use origami_core::{Code, Reported, Severity, Status};
use std::io::Write;

/// What an agent can do about a diagnostic.
fn hint(code: Code) -> &'static str {
    match code {
        Code::E001 => "Fix: close the literal, string template or block comment.",
        Code::E002 => "Fix: balance the brackets.",
        Code::E003 => "Fix: end the statement with a period.",
        Code::E004 => "Fix: match the block statements (e.g. IF … ENDIF).",
        Code::E005 => "Fix: save the file as UTF-8.",
        Code::E010 => "Fix: pass --language or use an abapGit file name.",
        Code::E011 => "Not a supported file type; skip it.",
        Code::E020 => "Fix the configuration (origami.toml).",
        Code::E021 => "Fix the command line (see origami --help).",
        Code::E030 => "Check that the file exists and is readable.",
        Code::E031 => "Check that the file is writable.",
        Code::W001 => "The unit was kept as written; no action needed.",
        Code::W002 => "The unit was kept as written (255-character limit); no action needed.",
        Code::W004 => "The unit was kept as written (nesting depth); no action needed.",
        Code::W101 => "Remove or fix the key in origami.toml.",
        Code::X001 | Code::X002 => {
            "Internal error; the file was not changed. Report it with the file."
        }
        _ => "",
    }
}

fn line(prefix: &str, f: &FileReport, d: &Reported) -> String {
    let mut s = format!(
        "{prefix} {}:{}:{} {} {}",
        super::escape_controls(&f.path),
        d.range.start.line,
        d.range.start.column,
        d.code.as_str(),
        d.message.trim_end_matches('.')
    );
    s.push('.');
    let h = match d.code {
        // The value came from a flag, not from origami.toml.
        Code::E020 if f.path == "<args>" => "Fix the flag value on the command line.",
        code => hint(code),
    };
    if !h.is_empty() {
        s.push(' ');
        s.push_str(h);
    }
    if f.status == Status::Error && d.severity >= Severity::Error && !h.contains("not changed") {
        s.push_str(" The file was not changed.");
    }
    s
}

pub fn write(run: &Run, out: &mut dyn Write) -> std::io::Result<()> {
    let mut files: Vec<&FileReport> = run.files.iter().collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let sources: Vec<&&FileReport> = files
        .iter()
        .filter(|f| f.status != Status::Skipped)
        .collect();
    let changed: Vec<&str> = sources
        .iter()
        .filter(|f| f.changed)
        .map(|f| f.path.as_str())
        .collect();
    let errors = sources.iter().filter(|f| f.status == Status::Error).count();
    let all: Vec<&Reported> = files.iter().flat_map(|f| &f.diagnostics).collect();
    let w003 = all.iter().filter(|d| d.code == Code::W003).count();
    let warnings = all
        .iter()
        .filter(|d| d.severity == Severity::Warning && d.code != Code::W003)
        .count();
    // A single source formatted without findings (an editor or agent piping
    // through stdin): nothing to say, as in the human format.
    if sources.len() <= 1 && !run.check && all.is_empty() {
        return Ok(());
    }
    let verb = if run.check {
        "would change"
    } else {
        "formatted"
    };
    let n = sources.len();
    writeln!(
        out,
        "origami: {n} {}, {} {verb}, {errors} with errors, {warnings} warnings",
        if n == 1 { "file" } else { "files" },
        changed.len()
    )?;
    let mut left = run.max_diagnostics;
    for f in &files {
        for d in f.diagnostics.iter().filter(|d| d.code != Code::W003) {
            if left == 0 {
                break;
            }
            left -= 1;
            let prefix = if d.severity >= Severity::Error {
                "ERROR"
            } else {
                "WARN"
            };
            writeln!(out, "{}", line(prefix, f, d))?;
        }
        if f.changed && f.status != Status::Error {
            writeln!(
                out,
                "{} {}",
                if run.check { "CHANGE" } else { "FORMATTED" },
                super::escape_controls(&f.path)
            )?;
        }
    }
    if w003 > 0 {
        writeln!(
            out,
            "NOTE {w003} statements without grammar keep their own line breaks (W003); no action needed."
        )?;
    }
    let next = if errors > 0 {
        "NEXT: fix the errors above (those files were not changed), then run origami again."
            .to_string()
    } else if run.check && !changed.is_empty() {
        let shown: Vec<String> = changed
            .iter()
            .take(10)
            .map(|p| super::shell_quote(p).into_owned())
            .collect();
        let more = if changed.len() > 10 { " …" } else { "" };
        // A name with control characters is shown escaped, not runnable.
        format!(
            "NEXT: run `origami {}{more}` to apply the formatting.",
            super::escape_controls(&shown.join(" "))
        )
    } else {
        "NEXT: nothing to do.".to_string()
    };
    writeln!(out, "{next}")
}

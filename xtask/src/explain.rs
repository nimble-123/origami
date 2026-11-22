//! `cargo xtask explain <file>`: the notes origami gives for the line breaks
//! it introduces in one file.
//!
//! `cargo xtask explain --corpus`: runs the explain pass over the corpus and
//! fails if
//! (a) a break token belongs to more than one note, or a note has none;
//! (b) `explain().result` differs from `format()`;
//! (c) a rule ID in a note has no `Explanation:` line in STYLE.md (skipped
//!     while STYLE.md has no such lines);
//! (d) a width note spans fewer lines than it has breaks plus one (its
//!     breaks lie inside its construct, so the construct has more lines).
//! It reports counts per cause and per rule, including `Layout` notes.

use crate::corpus::{files, manifest};
use crate::root;
use origami_core::{BreakCause, LanguageHint, Note, Options, explain, format};
use std::collections::{BTreeMap, BTreeSet};

fn cause_name(n: &Note) -> &'static str {
    match n.cause {
        BreakCause::Width { .. } => "width",
        BreakCause::Rule => "rule",
        BreakCause::Comment { .. } => "comment",
        BreakCause::Layout => "layout",
    }
}

pub fn file(path: &str) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let ex = explain(
        &bytes,
        &LanguageHint::Path(path.to_string()),
        &Options::default(),
        &[],
    );
    for d in &ex.result.diagnostics {
        println!(
            "{} {}:{} {}",
            d.code.as_str(),
            d.range.start.line,
            d.range.start.column,
            d.message
        );
    }
    for n in &ex.notes {
        println!("{}-{}: {}", n.lines.start(), n.lines.end(), n.text);
    }
    println!(
        "{} notes ({})",
        ex.notes.len(),
        if ex.result.changed {
            "would change"
        } else {
            "unchanged"
        }
    );
    Ok(())
}

/// Rule IDs that have an `Explanation:` line in STYLE.md.
fn explained_rules() -> Result<BTreeSet<String>, String> {
    let style = std::fs::read_to_string(root().join("docs/STYLE.md")).map_err(|e| e.to_string())?;
    let mut rules = BTreeSet::new();
    let mut current = None;
    for line in style.lines() {
        if let Some(h) = line.strip_prefix("### ") {
            current = h.split(':').next().map(|s| s.trim().to_string());
        } else if line.starts_with("Explanation:")
            && let Some(r) = &current
        {
            rules.insert(r.clone());
        }
    }
    Ok(rules)
}

pub fn corpus() -> Result<(), String> {
    let m = manifest()?;
    let opts = Options::default();
    let explained = explained_rules()?;
    let mut failures = Vec::new();
    let mut by_cause: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_rule: BTreeMap<String, usize> = BTreeMap::new();
    let (mut sources, mut with_notes) = (0, 0);
    for r in &m.repo {
        for (rel, path) in files(r) {
            let name = format!("{}/{}", r.name, rel);
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            let hint = LanguageHint::Path(path.to_string_lossy().into_owned());
            let ex = explain(&bytes, &hint, &opts, &[]);
            sources += 1;
            if ex.result != format(&bytes, &hint, &opts) {
                failures.push(format!("(b) {name}: explain result differs from format"));
            }
            let mut seen = BTreeSet::new();
            for n in &ex.notes {
                if n.break_tokens.is_empty() {
                    failures.push(format!(
                        "(a) {name}:{}: note without breaks",
                        n.lines.start()
                    ));
                }
                for t in &n.break_tokens {
                    if !seen.insert(t.0) {
                        failures.push(format!(
                            "(a) {name}:{}: break in more than one note",
                            n.lines.start()
                        ));
                    }
                }
                if let Some(rule) = n.rule
                    && !explained.is_empty()
                    && !explained.contains(rule)
                {
                    failures.push(format!(
                        "(c) {name}: {rule} has no Explanation line in STYLE.md"
                    ));
                }
                if matches!(n.cause, BreakCause::Width { .. })
                    && ((n.lines.end() - n.lines.start()) as usize) < n.break_tokens.len()
                {
                    failures.push(format!(
                        "(d) {name}:{}: width breaks outside their construct",
                        n.lines.start()
                    ));
                }
                *by_cause.entry(cause_name(n)).or_default() += 1;
                *by_rule
                    .entry(n.rule.unwrap_or("(no rule)").to_string())
                    .or_default() += 1;
            }
            if !ex.notes.is_empty() {
                with_notes += 1;
            }
        }
    }
    let total: usize = by_cause.values().sum();
    println!("explain: {sources} files, {with_notes} with notes, {total} notes");
    for (c, n) in &by_cause {
        println!("  cause {c:<8} {n}");
    }
    for (r, n) in &by_rule {
        println!("  rule  {r:<16} {n}");
    }
    if explained.is_empty() {
        println!("  (c) skipped: STYLE.md has no Explanation lines yet");
    }
    if failures.is_empty() {
        Ok(())
    } else {
        for f in failures.iter().take(50) {
            eprintln!("{f}");
        }
        Err(format!("{} explain check(s) failed", failures.len()))
    }
}

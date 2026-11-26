//! The review view of `--review` (stdout): for each file that would change,
//! the changes with old and new line numbers, the changed characters
//! highlighted, and a note below each construct whose line breaks origami
//! introduced.
//!
//! ```text
//! zcl_travel.clas.abap  (1 change, 1 note)
//!  41 41 │   METHOD validate.
//!  42    │-    ls_res = zcl_check=>run( iv_id = ls_travel-id iv_date = sy-datum ).
//!     42 │+    ls_res = zcl_check=>run( iv_id   = ls_travel-id
//!     43 │+                             iv_date = sy-datum ).
//!        │   ↳ ABAP-CALL-001: one parameter per line, because …
//!  43 44 │   ENDMETHOD.
//! ```
//!
//! Without colors, changed characters are marked `[-…-]` (removed) and
//! `{+…+}` (added). Control characters from the source are escaped.

use super::escape_controls;
use origami_core::Explained;
use similar::{ChangeTag, DiffTag, TextDiff};
use std::collections::BTreeMap;

/// Lines of context around each change.
const CONTEXT: usize = 2;

struct Paint {
    on: bool,
}

impl Paint {
    fn sgr(&self, code: &str, s: &str) -> String {
        if self.on {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The view for one file, or an empty string if nothing changes.
pub fn render(path: &str, input: &[u8], e: &Explained, color: bool) -> String {
    if !e.result.changed || e.output_text == e.input_text {
        return String::new();
    }
    let p = Paint { on: color };
    let diff = TextDiff::from_lines(e.input_text.as_str(), e.output_text.as_str());
    let groups = diff.grouped_ops(CONTEXT);

    // Each note goes below the last added line inside its construct (else
    // the first added line after its start).
    let added: Vec<usize> = groups
        .iter()
        .flatten()
        .filter(|op| matches!(op.tag(), DiffTag::Insert | DiffTag::Replace))
        .flat_map(|op| op.new_range())
        .map(|n| n + 1)
        .collect();
    let mut anchored: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
    let mut loose = Vec::new();
    for n in &e.notes {
        let (start, end) = (*n.lines.start() as usize, *n.lines.end() as usize);
        // `added` is ascending: binary search, not a scan per note.
        let anchor = added[..added.partition_point(|&l| l <= end)]
            .last()
            .filter(|&&l| l >= start)
            .or_else(|| added.get(added.partition_point(|&l| l < start)));
        match anchor {
            Some(&l) => anchored.entry(l).or_default().push(&n.text),
            None => loose.push(n.text.as_str()),
        }
    }

    let width = e
        .input_text
        .lines()
        .count()
        .max(e.output_text.lines().count())
        .to_string()
        .len();
    let blank = " ".repeat(width);
    let bar = p.sgr("2", "│");
    let note_line = |text: &str| {
        format!(
            "{blank} {blank} {bar}   {}\n",
            p.sgr("36", &format!("↳ {}", escape_controls(text)))
        )
    };

    let mut flags = Vec::new();
    if input.starts_with(&[0xef, 0xbb, 0xbf]) {
        flags.push("BOM".to_string());
    }
    if input.windows(2).any(|w| w == b"\r\n") {
        flags.push("CRLF".to_string());
    }
    let mut header = vec![
        plural(groups.len(), "change", "changes"),
        plural(e.notes.len(), "note", "notes"),
    ];
    header.extend(flags);
    let mut out = format!(
        "{}  {}\n",
        p.sgr("1", &escape_controls(path)),
        p.sgr("2", &format!("({})", header.join(", ")))
    );

    for (g, group) in groups.iter().enumerate() {
        if g > 0 {
            out.push_str(&format!("{blank} {blank} {}\n", p.sgr("2", "⋮")));
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                let old = change
                    .old_index()
                    .map_or(blank.clone(), |i| format!("{:>width$}", i + 1));
                let new = change
                    .new_index()
                    .map_or(blank.clone(), |i| format!("{:>width$}", i + 1));
                let (sign, line_code, open, close) = match change.tag() {
                    ChangeTag::Equal => (" ", "", "", ""),
                    ChangeTag::Delete => ("-", "31", "[-", "-]"),
                    ChangeTag::Insert => ("+", "32", "{+", "+}"),
                };
                let mut text = String::new();
                for (emphasized, value) in change.iter_strings_lossy() {
                    let value = value.trim_end_matches(['\n', '\r']);
                    if value.is_empty() {
                        continue;
                    }
                    let value = escape_controls(value);
                    if change.tag() == ChangeTag::Equal {
                        text.push_str(&value);
                    } else if emphasized {
                        text.push_str(&if color {
                            p.sgr(&format!("{line_code};7"), &value)
                        } else {
                            format!("{open}{value}{close}")
                        });
                    } else {
                        text.push_str(&p.sgr(line_code, &value));
                    }
                }
                let sign = if line_code.is_empty() {
                    sign.to_string()
                } else {
                    p.sgr(line_code, sign)
                };
                out.push_str(&format!(
                    "{} {} {bar}{sign}{text}\n",
                    p.sgr("2", &old),
                    p.sgr("2", &new)
                ));
                if change.tag() == ChangeTag::Insert
                    && let Some(i) = change.new_index()
                    && let Some(texts) = anchored.remove(&(i + 1))
                {
                    for t in texts {
                        out.push_str(&note_line(t));
                    }
                }
            }
        }
    }
    for t in loose.into_iter().chain(anchored.into_values().flatten()) {
        out.push_str(&note_line(t));
    }
    out.push('\n');
    out
}

//! Unified diff for `--check --diff`.

use similar::TextDiff;

pub fn unified(path: &str, old: &[u8], new: &[u8]) -> String {
    let old = String::from_utf8_lossy(old);
    let new = String::from_utf8_lossy(new);
    TextDiff::from_lines(old.as_ref(), new.as_ref())
        .unified_diff()
        .header(&format!("a/{path}"), &format!("b/{path}"))
        .to_string()
}

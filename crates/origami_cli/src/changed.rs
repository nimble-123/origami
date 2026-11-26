//! `--changed [REF]`: the supported files git reports as changed against REF
//! (working tree, staged or not), with the changed line ranges, plus
//! untracked files (formatted in full). Paths are relative to the current
//! directory, as git prints them with `--relative`.

use origami_core::detect::{FileKind, by_file_name};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Line ranges to format; `None` formats the whole file.
pub type Ranges = Option<Vec<(u32, u32)>>;

fn git(args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(["-c", "core.quotePath=false"])
        .args(args)
        .output()
        .map_err(|e| format!("--changed needs git: {e}"))?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr);
        let first = msg.lines().next().unwrap_or("").trim();
        return Err(format!("--changed: git {}: {first}", args[0]));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn supported(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    by_file_name(name) != FileKind::Unsupported
}

/// The path in a `+++ ` header: git writes a name with a space followed by a
/// tab, and a name with `"`, `\`, a tab or a control character in C quotes.
/// `None` for `/dev/null` (a deleted file) and for a name without the `b/`
/// prefix that `files` asks for.
fn header_path(p: &str) -> Option<String> {
    let name = match p.strip_prefix('"') {
        Some(quoted) => unquote(quoted),
        None => p.strip_suffix('\t').unwrap_or(p).to_string(),
    };
    name.strip_prefix("b/").map(str::to_string)
}

/// The text of a C-quoted name up to its closing quote.
fn unquote(s: &str) -> String {
    let mut out = Vec::new();
    let mut it = s.bytes().peekable();
    while let Some(c) = it.next() {
        match c {
            b'"' => break,
            b'\\' => match it.next() {
                Some(d @ b'0'..=b'7') => {
                    let mut n = u32::from(d - b'0');
                    for _ in 0..2 {
                        match it.next_if(|o| (b'0'..=b'7').contains(o)) {
                            Some(o) => n = n * 8 + u32::from(o - b'0'),
                            None => break,
                        }
                    }
                    out.push(n as u8);
                }
                Some(b'a') => out.push(7),
                Some(b'b') => out.push(8),
                Some(b'f') => out.push(12),
                Some(b'n') => out.push(b'\n'),
                Some(b'r') => out.push(b'\r'),
                Some(b't') => out.push(b'\t'),
                Some(b'v') => out.push(11),
                Some(other) => out.push(other),
                None => break,
            },
            _ => out.push(c),
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A hunk header `@@ -a[,b] +c[,d] @@` as (old count, new start, new count).
fn hunk(h: &str) -> Option<(u32, u32, u32)> {
    let count = |t: &str, default| match t.split_once(',') {
        Some((_, n)) => n.parse().ok(),
        None => Some(default),
    };
    let mut tokens = h.split_whitespace();
    let old = tokens.next()?.strip_prefix('-')?;
    let new = tokens.next()?.strip_prefix('+')?;
    let start = new.split(',').next()?.parse().ok()?;
    Some((count(old, 1)?, start, count(new, 1)?))
}

/// Parses `git diff -U0 --src-prefix=a/ --dst-prefix=b/` output into new-side
/// line ranges per file.
pub fn parse_diff(diff: &str) -> BTreeMap<String, Vec<(u32, u32)>> {
    let mut out: BTreeMap<String, Vec<(u32, u32)>> = BTreeMap::new();
    let mut current: Option<String> = None;
    // Removed and added lines of the hunk still to come. They are counted
    // off, because an added line `++ b/x` looks like a file header.
    let mut left = (0u32, 0u32);
    for line in diff.lines() {
        if left != (0, 0) {
            let in_hunk = match line.as_bytes().first() {
                Some(b'-') if left.0 > 0 => {
                    left.0 -= 1;
                    true
                }
                Some(b'+') if left.1 > 0 => {
                    left.1 -= 1;
                    true
                }
                Some(b'\\') => true,
                // Not what the hunk header promised: read it as a header.
                _ => {
                    left = (0, 0);
                    false
                }
            };
            if in_hunk {
                continue;
            }
        }
        if let Some(p) = line.strip_prefix("+++ ") {
            current = header_path(p);
            continue;
        }
        let (Some(file), Some(h)) = (&current, line.strip_prefix("@@ ")) else {
            continue;
        };
        let Some((removed, c, d)) = hunk(h) else {
            continue;
        };
        left = (removed, d);
        // A pure deletion (d = 0) touches the lines around it.
        let range = if d == 0 {
            (c.max(1), c + 1)
        } else {
            (c, c + d - 1)
        };
        out.entry(file.clone()).or_default().push(range);
    }
    out
}

/// The commit `rev` names, as a full object name. A revision that starts with
/// `-` would be read by git as an option (`--output=FILE` overwrites FILE),
/// so it is refused before git sees it, and git is given the resolved name.
fn resolve(rev: &str) -> Result<String, String> {
    if rev.starts_with('-') {
        return Err(format!(
            "--changed: `{rev}` is not a git revision (it starts with `-`)"
        ));
    }
    let out = Command::new("git")
        .args(["rev-parse", "--verify", "--quiet"])
        .arg(format!("{rev}^{{commit}}"))
        .output()
        .map_err(|e| format!("--changed needs git: {e}"))?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() && !name.is_empty() {
        Ok(name)
    } else {
        Err(format!("--changed: `{rev}` is not a known git revision"))
    }
}

/// Fails with one line unless the current directory is inside a git work tree.
fn in_work_tree() -> Result<(), String> {
    let out = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map_err(|e| format!("--changed needs git: {e}"))?;
    if out.status.success() && out.stdout.trim_ascii() == b"true" {
        Ok(())
    } else {
        Err("--changed: the current directory is not inside a git work tree".into())
    }
}

pub fn files(rev: &str, paths: &[PathBuf]) -> Result<Vec<(PathBuf, Ranges)>, String> {
    in_work_tree()?;
    let rev = resolve(rev)?;
    let specs: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
    let mut diff_args = vec![
        "diff",
        "-U0",
        "--no-color",
        "--no-ext-diff",
        "--relative",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        &rev,
        "--",
    ];
    diff_args.extend(specs.iter().map(String::as_str));
    let diff = git(&diff_args)?;
    let mut result: Vec<(PathBuf, Ranges)> = parse_diff(&diff)
        .into_iter()
        .filter(|(f, _)| supported(f))
        .map(|(f, r)| (PathBuf::from(f), Some(r)))
        .collect();
    let mut ls_args = vec!["ls-files", "-z", "--others", "--exclude-standard", "--"];
    ls_args.extend(specs.iter().map(String::as_str));
    for f in git(&ls_args)?.split('\0').filter(|f| supported(f)) {
        result.push((PathBuf::from(f), None));
    }
    result.retain(|(f, _)| f.is_file());
    // The same files a directory walk finds: .gitignore, .origamiignore and
    // hidden files apply; a file named in `paths` is always taken.
    let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let walked: HashSet<PathBuf> = crate::walk::files(&[PathBuf::from(".")])
        .iter()
        .chain(paths.iter().filter(|p| p.is_file()))
        .map(|p| canonical(p))
        .collect();
    result.retain(|(f, _)| walked.contains(&canonical(f)));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_names_are_unquoted_and_untabbed() {
        assert_eq!(header_path("b/a.abap").as_deref(), Some("a.abap"));
        assert_eq!(header_path("b/a b.abap\t").as_deref(), Some("a b.abap"));
        assert_eq!(
            header_path(r#""b/q\"u\\o\tt\303\244.abap""#).as_deref(),
            Some("q\"u\\o\tt\u{e4}.abap")
        );
        assert_eq!(header_path("/dev/null"), None);
        assert_eq!(header_path("a.abap"), None);
    }

    #[test]
    fn an_added_line_that_looks_like_a_header_stays_content() {
        let diff = "\
diff --git a/x.prog.abap b/x.prog.abap
--- a/x.prog.abap
+++ b/x.prog.abap
@@ -1,2 +1,3 @@
-old
\\ No newline at end of file
+++ b/evil.prog.abap
+new
+--- a/z
@@ -9 +10 @@
-a
+b
";
        let got = parse_diff(diff);
        assert_eq!(
            got.keys().map(String::as_str).collect::<Vec<_>>(),
            ["x.prog.abap"]
        );
        assert_eq!(got["x.prog.abap"], [(1, 3), (10, 10)]);
    }

    #[test]
    fn deletions_touch_the_lines_around_them() {
        let diff = "+++ b/x.prog.abap\n@@ -4,2 +3,0 @@\n-a\n-b\n";
        assert_eq!(parse_diff(diff)["x.prog.abap"], [(3, 4)]);
    }
}

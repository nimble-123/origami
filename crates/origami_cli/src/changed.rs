//! `--changed [REF]`: the supported files git reports as changed against REF
//! (working tree, staged or not), with the changed line ranges, plus
//! untracked files (formatted in full). Paths are relative to the current
//! directory, as git prints them with `--relative`.

use origami_core::detect::{FileKind, by_file_name};
use std::collections::BTreeMap;
use std::path::PathBuf;
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
        return Err(format!("--changed: git {}: {}", args[0], msg.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn supported(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    by_file_name(name) != FileKind::Unsupported
}

/// Parses `git diff -U0` output into new-side line ranges per file.
pub fn parse_diff(diff: &str) -> BTreeMap<String, Vec<(u32, u32)>> {
    let mut out: BTreeMap<String, Vec<(u32, u32)>> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in diff.lines() {
        if let Some(p) = line.strip_prefix("+++ ") {
            current = p.strip_prefix("b/").map(str::to_string);
            continue;
        }
        let (Some(file), Some(h)) = (&current, line.strip_prefix("@@ ")) else {
            continue;
        };
        // @@ -a[,b] +c[,d] @@
        let Some(plus) = h.split_whitespace().find(|t| t.starts_with('+')) else {
            continue;
        };
        let mut it = plus[1..].split(',');
        let c: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        let d: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(1);
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

pub fn files(rev: &str, paths: &[PathBuf]) -> Result<Vec<(PathBuf, Ranges)>, String> {
    let rev = resolve(rev)?;
    let specs: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
    let mut diff_args = vec![
        "diff",
        "-U0",
        "--no-color",
        "--no-ext-diff",
        "--relative",
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
    let mut ls_args = vec!["ls-files", "--others", "--exclude-standard", "--"];
    ls_args.extend(specs.iter().map(String::as_str));
    for f in git(&ls_args)?.lines().filter(|f| supported(f)) {
        result.push((PathBuf::from(f), None));
    }
    result.retain(|(f, _)| f.is_file());
    Ok(result)
}

//! Corpus tasks.
//!
//! `run` formats every supported file twice in memory and asserts: no E-codes,
//! no W001 and no X-codes, and idempotency. It computes the fallback rate per
//! language (statements with W003 or W004, divided by all statements; in
//! identity mode every statement counts as unknown), separately for `modern`
//! repositories, and fails when a rate rises above `corpus/baseline.json`.
//! `bless` rewrites the baseline.
//!
//! `hashes` (`corpus run --hashes FILE`) writes one line per file and setting
//! with a hash of the formatted output, for checking that a change leaves all
//! output byte for byte unchanged (design origami-explain-review, goal G3).

use crate::root;
use origami_core::detect::{FileKind, by_file_name};
use origami_core::{Code, LanguageHint, Options, Status, format};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub repo: Vec<Repo>,
}

#[derive(Debug, Deserialize)]
pub struct Repo {
    pub name: String,
    pub url: String,
    pub commit: String,
    #[allow(dead_code)]
    pub spdx: String,
    pub include: Vec<String>,
    pub modern: bool,
    #[serde(default)]
    pub exclude: Vec<String>,
}

pub fn manifest() -> Result<Manifest, String> {
    let text =
        std::fs::read_to_string(root().join("corpus/manifest.toml")).map_err(|e| e.to_string())?;
    toml::from_str(&text).map_err(|e| e.to_string())
}

fn git(dir: &Path, args: &[&str]) -> Result<(), String> {
    let ok = Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .map_err(|e| e.to_string())?;
    if ok.success() {
        Ok(())
    } else {
        Err(format!(
            "git {} failed in {}",
            args.join(" "),
            dir.display()
        ))
    }
}

pub fn fetch() -> Result<(), String> {
    let m = manifest()?;
    for r in &m.repo {
        let dir = root().join("corpus/repos").join(&r.name);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        if !dir.join(".git").exists() {
            git(&dir, &["init", "-q"])?;
            git(&dir, &["remote", "add", "origin", &r.url])?;
        }
        git(&dir, &["fetch", "-q", "--depth", "1", "origin", &r.commit])?;
        git(&dir, &["checkout", "-q", "--force", &r.commit])?;
        eprintln!("fetched {} @ {}", r.name, &r.commit[..12]);
    }
    Ok(())
}

/// Minimal glob match supporting `**` (any path) and `*` (any name part).
pub fn glob(pattern: &str, path: &str) -> bool {
    fn m(p: &[u8], s: &[u8]) -> bool {
        match (p.first(), s.first()) {
            (None, None) => true,
            (Some(b'*'), _) if p.get(1) == Some(&b'*') => {
                let rest = if p.get(2) == Some(&b'/') {
                    &p[3..]
                } else {
                    &p[2..]
                };
                (0..=s.len()).any(|k| m(rest, &s[k..]))
            }
            (Some(b'*'), _) => (0..=s.len())
                .take_while(|&k| k == 0 || s[k - 1] != b'/')
                .any(|k| m(&p[1..], &s[k..])),
            (Some(a), Some(b)) if a == b => m(&p[1..], &s[1..]),
            _ => false,
        }
    }
    m(pattern.as_bytes(), path.as_bytes())
}

/// Supported files of one repository, relative paths sorted.
pub fn files(r: &Repo) -> Vec<(String, PathBuf)> {
    let base = root().join("corpus/repos").join(&r.name);
    let mut out = Vec::new();
    let mut stack = vec![base.clone()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let rel = p
                .strip_prefix(&base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let name = p.file_name().unwrap().to_string_lossy();
            if by_file_name(&name) == FileKind::Unsupported {
                continue;
            }
            if r.include.iter().any(|g| glob(g, &rel)) && !r.exclude.iter().any(|g| glob(g, &rel)) {
                out.push((rel, p));
            }
        }
    }
    out.sort();
    out
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct LangStats {
    pub files: u64,
    pub statements: u64,
    pub fallback: u64,
    pub fallback_rate: f64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Baseline {
    pub all: BTreeMap<String, LangStats>,
    pub modern: BTreeMap<String, LangStats>,
}

pub fn run(bless: bool) -> Result<(), String> {
    let m = manifest()?;
    let opts = Options::default();
    let mut report = Baseline::default();
    let mut failures = Vec::new();
    let mut total_files = 0u64;
    for r in &m.repo {
        let list = files(r);
        if list.is_empty() {
            return Err(format!(
                "no files for {} — run `cargo xtask corpus fetch`",
                r.name
            ));
        }
        for (rel, path) in list {
            total_files += 1;
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            let name = path.to_string_lossy().into_owned();
            let res = format(&bytes, &LanguageHint::Path(name.clone()), &opts);
            let label = format!("{}/{}", r.name, rel);
            for d in &res.diagnostics {
                let bad = matches!(
                    d.severity,
                    origami_core::Severity::Error | origami_core::Severity::Internal
                ) || d.code == Code::W001;
                if bad {
                    failures.push(format!(
                        "{label}:{}:{}: {} {}",
                        d.range.start.line,
                        d.range.start.column,
                        d.code.as_str(),
                        d.message
                    ));
                }
            }
            if res.status == Status::Error {
                continue;
            }
            let out = res.output.clone().unwrap_or_default();
            let again = format(&out, &LanguageHint::Path(name), &opts);
            if again.output.as_deref() != Some(&out[..]) {
                failures.push(format!("{label}: not idempotent"));
            }
            let lang = res
                .language
                .map(|l| l.as_str().to_string())
                .unwrap_or_default();
            let fallback = if res.language.is_none_or(origami_core::identity_mode) {
                res.stats.statements
            } else {
                res.diagnostics
                    .iter()
                    .filter(|d| matches!(d.code, Code::W003 | Code::W004))
                    .count() as u32
            };
            for bucket in [
                Some(&mut report.all),
                r.modern.then_some(&mut report.modern),
            ]
            .into_iter()
            .flatten()
            {
                let s = bucket.entry(lang.clone()).or_default();
                s.files += 1;
                s.statements += res.stats.statements as u64;
                s.fallback += fallback as u64;
            }
        }
    }
    for bucket in [&mut report.all, &mut report.modern] {
        for s in bucket.values_mut() {
            s.fallback_rate = if s.statements == 0 {
                0.0
            } else {
                s.fallback as f64 / s.statements as f64
            };
        }
    }

    println!("corpus: {total_files} files");
    println!(
        "{:<6} {:>7} {:>10} {:>10} {:>8}",
        "lang", "files", "statements", "fallback", "rate"
    );
    for (k, s) in &report.all {
        println!(
            "{k:<6} {:>7} {:>10} {:>10} {:>7.1}%",
            s.files,
            s.statements,
            s.fallback,
            s.fallback_rate * 100.0
        );
    }
    let out_path = root().join("target/corpus-report.json");
    std::fs::create_dir_all(out_path.parent().unwrap()).ok();
    std::fs::write(&out_path, serde_json::to_string_pretty(&report).unwrap())
        .map_err(|e| e.to_string())?;

    if !failures.is_empty() {
        for f in failures.iter().take(50) {
            eprintln!("FAIL {f}");
        }
        return Err(format!("{} corpus failure(s)", failures.len()));
    }

    let baseline_path = root().join("corpus/baseline.json");
    if bless {
        std::fs::write(
            &baseline_path,
            serde_json::to_string_pretty(&report).unwrap() + "\n",
        )
        .map_err(|e| e.to_string())?;
        println!("baseline written to corpus/baseline.json");
        return Ok(());
    }
    let baseline: Baseline = match std::fs::read_to_string(&baseline_path) {
        Ok(t) => serde_json::from_str(&t).map_err(|e| e.to_string())?,
        Err(_) => {
            return Err("corpus/baseline.json missing — run `cargo xtask corpus bless`".into());
        }
    };
    let mut regressions = Vec::new();
    for (scope, now, before) in [
        ("all", &report.all, &baseline.all),
        ("modern", &report.modern, &baseline.modern),
    ] {
        for (lang, s) in now {
            if let Some(b) = before.get(lang)
                && s.fallback_rate > b.fallback_rate + 1e-9
            {
                regressions.push(format!(
                    "{scope}/{lang}: fallback rate {:.2}% > baseline {:.2}%",
                    s.fallback_rate * 100.0,
                    b.fallback_rate * 100.0
                ));
            }
        }
    }
    if !regressions.is_empty() {
        return Err(regressions.join("\n"));
    }
    println!("corpus OK: no failures, idempotent, no fallback regression");
    Ok(())
}

/// 64-bit FNV-1a; only compared for equality.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// `corpus run --hashes FILE`: for every corpus file, the hash of the
/// formatted output at the default settings and at width 80, indent 4,
/// lower-case keywords (`ERR <code>` when formatting fails), sorted. Also
/// checks idempotency at the non-default setting.
pub fn hashes(out_file: &str) -> Result<(), String> {
    let m = manifest()?;
    let default = Options::default();
    let mut variant = Options::default();
    variant.lang.line_width = 80;
    variant.lang.indent_width = 4;
    variant.lang.abap_keyword_case = origami_core::Case::Lower;
    let mut lines = Vec::new();
    let mut not_idempotent = Vec::new();
    for r in &m.repo {
        for (rel, path) in files(r) {
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            let hint = LanguageHint::Path(path.to_string_lossy().into_owned());
            for (name, opts) in [("default", &default), ("w80-i4-lower", &variant)] {
                let res = format(&bytes, &hint, opts);
                if origami_core::explain(&bytes, &hint, opts, &[]).result != res {
                    not_idempotent
                        .push(format!("{}/{} (explain differs from format)", r.name, rel));
                }
                let value = match &res.output {
                    Some(out) if res.status != Status::Error => format!("{:016x}", fnv1a(out)),
                    _ => format!(
                        "ERR {}",
                        res.diagnostics
                            .iter()
                            .find(|d| d.code.as_str().starts_with(['E', 'X']))
                            .map(|d| d.code.as_str())
                            .unwrap_or("?")
                    ),
                };
                if name != "default"
                    && let Some(out) = &res.output
                    && format(out, &hint, opts).output.as_deref() != Some(&out[..])
                {
                    not_idempotent.push(format!("{}/{}", r.name, rel));
                }
                lines.push(format!("{value} {name} {}/{}", r.name, rel));
            }
        }
    }
    lines.sort();
    let path = root().join(out_file);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
    println!(
        "hashes: {} lines written to {}",
        lines.len(),
        path.display()
    );
    if not_idempotent.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "not idempotent at width 80 / indent 4 / lower case: {}",
            not_idempotent.join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::glob;

    #[test]
    fn globs() {
        assert!(glob("src/**", "src/a/b.abap"));
        assert!(glob("src/**", "src/b.abap"));
        assert!(!glob("src/**", "test/b.abap"));
        assert!(glob("src/*.abap", "src/b.abap"));
        assert!(!glob("src/*.abap", "src/x/b.abap"));
        assert!(glob("**/zz_*", "src/x/zz_y"));
    }
}

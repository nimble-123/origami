//! The single snapshot harness for all STYLE rules.
//!
//! Reads the rule IDs and `Milestone:` lines from `docs/STYLE.md`. For every
//! rule up to `CURRENT_MILESTONE` it checks `format(input) == expected`,
//! `format(expected) == expected`, that no W001 or X001 is reported and that
//! `explain` returns the same result as `format`. It
//! also checks that every rule has a directory and every directory a rule.

use origami_core::{Code, LanguageHint, Options, explain, format};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const CURRENT_MILESTONE: u8 = 4;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn valid_id(id: &str) -> bool {
    let parts: Vec<&str> = id.split('-').collect();
    parts.len() == 3
        && ["GEN", "ABAP", "CDS", "DDLS", "DDLX", "DCL", "BDEF", "SRVD"].contains(&parts[0])
        && !parts[1].is_empty()
        && parts[1].bytes().all(|b| b.is_ascii_uppercase())
        && parts[2].len() == 3
        && parts[2].bytes().all(|b| b.is_ascii_digit())
}

/// Rule ID → milestone, from STYLE.md.
fn rules() -> BTreeMap<String, u8> {
    let text = std::fs::read_to_string(root().join("docs/STYLE.md")).expect("docs/STYLE.md");
    let lines: Vec<&str> = text.lines().collect();
    let mut out = BTreeMap::new();
    for (k, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("### ") else {
            continue;
        };
        let Some((id, _title)) = rest.split_once(": ") else {
            continue;
        };
        if !valid_id(id) {
            continue;
        }
        let milestone = lines[k + 1..]
            .iter()
            .find(|l| !l.trim().is_empty())
            .and_then(|l| l.strip_prefix("Milestone: M"))
            .and_then(|n| n.trim().parse().ok())
            .unwrap_or_else(|| panic!("rule {id} has no `Milestone: M<n>` line"));
        assert!(
            out.insert(id.to_string(), milestone).is_none(),
            "duplicate rule {id}"
        );
    }
    out
}

fn find(dir: &Path, prefix: &str) -> PathBuf {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.file_name().unwrap().to_string_lossy().starts_with(prefix))
        .unwrap_or_else(|| panic!("{} has no {prefix}* file", dir.display()))
}

#[test]
fn style_rules() {
    let rules = rules();
    assert!(!rules.is_empty(), "no rules found in docs/STYLE.md");
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/rules");
    let dirs: Vec<String> = std::fs::read_dir(&base)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    for d in &dirs {
        assert!(
            rules.contains_key(d),
            "rule directory {d} has no STYLE entry"
        );
    }
    let opts = Options::default();
    let mut ran = 0;
    for (id, milestone) in &rules {
        if *milestone > CURRENT_MILESTONE {
            continue;
        }
        let dir = base.join(id);
        assert!(dir.is_dir(), "rule {id} has no test directory");
        let input = find(&dir, "input.");
        let expected = find(&dir, "expected.");
        let in_bytes = std::fs::read(&input).unwrap();
        let exp_bytes = std::fs::read(&expected).unwrap();
        for (label, bytes) in [("input", &in_bytes), ("expected", &exp_bytes)] {
            let hint = LanguageHint::Path(input.to_string_lossy().into_owned());
            let r = format(bytes, &hint, &opts);
            assert_eq!(
                explain(bytes, &hint, &opts, &[]).result,
                r,
                "{id}: explain differs from format on {label}"
            );
            assert!(
                !r.has(Code::W001) && !r.has(Code::X001),
                "{id}: self-check problem on {label}"
            );
            let out = r
                .output
                .unwrap_or_else(|| panic!("{id}: no output for {label}: {:?}", r.diagnostics));
            assert_eq!(
                String::from_utf8_lossy(&out),
                String::from_utf8_lossy(&exp_bytes),
                "{id}: {label} does not format to expected"
            );
        }
        ran += 1;
    }
    assert!(ran > 0);
}

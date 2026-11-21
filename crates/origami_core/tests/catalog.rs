//! The abaplint statement catalog (`corpus/catalog/`): every ABAP example
//! statement and CDS source from abaplint's tests, up to SAP_BASIS 8.16.
//! Checks: no internal error, idempotent output, no structural error other
//! than an open block (a single `IF …` has no `ENDIF`), and at most
//! `MAX_W003` statements without a grammar.

use origami_core::{Code, Language, LanguageHint, Options, format};
use std::path::Path;

/// Statements without a grammar today: 18 dynpro flow-logic statements
/// (`PROCESS …`, `FIELD …`, `CHAIN`), which are not ABAP source, plus a few
/// contrived cases (a variable named `mod`, dynamic `MERGE (x)`).
const MAX_W003: usize = 21;

fn catalog(name: &str) -> serde_json::Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/catalog")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn check(label: &str, lang: Language, text: &str) -> Vec<Code> {
    let hint = LanguageHint::Explicit(lang);
    let opts = Options::default();
    let r = format(format!("{text}\n").as_bytes(), &hint, &opts);
    let codes: Vec<Code> = r.diagnostics.iter().map(|d| d.code).collect();
    assert!(
        !r.has(Code::X001) && !r.has(Code::X002),
        "{label}: internal error on {text:?}"
    );
    if let Some(out) = r.output {
        let again = format(&out, &hint, &opts);
        assert_eq!(
            again.output.as_deref(),
            Some(&out[..]),
            "{label}: not idempotent on {text:?}"
        );
    }
    codes
}

#[test]
fn abap_statement_catalog() {
    let mut w003 = Vec::new();
    for e in catalog("abaplint-abap.json").as_array().unwrap() {
        let ty = e["type"].as_str().unwrap();
        let abap = e["abap"].as_str().unwrap();
        let codes = check(ty, Language::Abap, abap);
        let structural: Vec<_> = codes
            .iter()
            .filter(|c| c.as_str().starts_with('E') && **c != Code::E004)
            .collect();
        assert!(structural.is_empty(), "{ty}: {structural:?} on {abap:?}");
        if codes.contains(&Code::W003) {
            w003.push(format!(
                "{ty}: {}",
                abap.split_whitespace().collect::<Vec<_>>().join(" ")
            ));
        }
    }
    assert!(
        w003.len() <= MAX_W003,
        "{} statements without grammar (max {MAX_W003}):\n{}",
        w003.len(),
        w003.join("\n")
    );
}

#[test]
fn cds_catalog() {
    for src in catalog("abaplint-cds.json").as_array().unwrap() {
        let src = src.as_str().unwrap();
        let codes = check("cds", Language::Ddls, src);
        assert!(
            !codes.iter().any(|c| c.as_str().starts_with('E')),
            "cds: {codes:?} on {src:?}"
        );
    }
}

//! In-process benchmarks. Per language, the corpus file at the median line
//! count (when `cargo xtask corpus fetch` has run), plus a generated
//! 50,000-line ABAP file.

use criterion::{Criterion, criterion_group, criterion_main};
use origami_core::{LanguageHint, Options, format};
use std::hint::black_box;
use std::path::Path;

fn generated_abap(lines: usize) -> String {
    let mut s = String::from(
        "CLASS lcl DEFINITION.\n  PUBLIC SECTION.\n    METHODS run.\nENDCLASS.\nCLASS lcl IMPLEMENTATION.\n  METHOD run.\n",
    );
    for k in 0..lines {
        s.push_str(&format!(
            "    DATA(lv_{k}) = |Value {{ lv_{k} }}| && 'x'. \" comment\n"
        ));
    }
    s.push_str("  ENDMETHOD.\nENDCLASS.\n");
    s
}

fn median_corpus_file(ext: &str) -> Option<(String, Vec<u8>)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/repos");
    let mut files = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).ok()?.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.to_string_lossy().ends_with(ext) {
                let bytes = std::fs::read(&p).ok()?;
                let lines = bytes.iter().filter(|&&b| b == b'\n').count();
                files.push((lines, p.to_string_lossy().into_owned(), bytes));
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let (_, name, bytes) = files.get(files.len() / 2)?.clone();
    Some((name, bytes))
}

fn benches(c: &mut Criterion) {
    let opts = Options::default();
    for ext in [
        ".clas.abap",
        ".ddls.asddls",
        ".ddlx.asddlxs",
        ".dcls.asdcls",
        ".bdef.asbdef",
        ".srvd.srvdsrv",
    ] {
        if let Some((name, bytes)) = median_corpus_file(ext) {
            let hint = LanguageHint::Path(name);
            c.bench_function(&format!("median{ext}"), |b| {
                b.iter(|| format(black_box(&bytes), &hint, &opts))
            });
        }
    }
    let big = generated_abap(50_000);
    let hint = LanguageHint::Explicit(origami_core::Language::Abap);
    c.bench_function("generated_50k_abap", |b| {
        b.iter(|| format(black_box(big.as_bytes()), &hint, &opts))
    });
}

criterion_group!(group, benches);
criterion_main!(group);

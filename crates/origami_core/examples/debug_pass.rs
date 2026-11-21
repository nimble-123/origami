//! Development aid: prints the first-pass output of a file (before the
//! self-check) and the token mismatches. Usage:
//! cargo run -p origami_core --example debug_pass -- <file>
use origami_core::detect::{LanguageHint, detect};
use origami_syntax::{LangOptions, Language, SourceText, equivalent};
use std::collections::BTreeSet;

fn main() {
    let path = std::env::args().nth(1).expect("file");
    let bytes = std::fs::read(&path).unwrap();
    let src = SourceText::decode(&bytes).unwrap();
    let lang = detect(&src, &LanguageHint::Path(path.clone())).unwrap();
    let e = BTreeSet::new();
    let o = LangOptions::default();
    let r = match lang {
        Language::Abap => origami_abap::format(&src, &o, &e, &e, false),
        l => origami_cds::format(&src, l, &o, &e, &e, false),
    };
    println!("{}", r.output);
    let out = SourceText::from_normalized(r.output.clone());
    let relexed = match lang {
        Language::Abap => origami_abap::lex(&out).unwrap(),
        l => origami_cds::lex(&out, l).unwrap(),
    };
    match equivalent(&r.input_tokens, src.text(), &relexed, &r.output) {
        Ok(()) => eprintln!("equivalent"),
        Err(ms) => {
            for m in ms.iter().take(10) {
                let t = &r.input_tokens.tokens[m.input_index.0 as usize];
                let line = src.text()[..t.start as usize].matches('\n').count() + 1;
                eprintln!(
                    "{:?} at input line {line}: {:?}",
                    m.kind,
                    t.text(src.text())
                );
            }
        }
    }
}

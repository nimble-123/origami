//! `oracle`: runs the pinned abaplint lexer (tools/abaplint-oracle) on the
//! input and the output of every ABAP corpus file and compares the token
//! sequences: type plus text, with words compared case-insensitively outside
//! strings, templates and comments. This catches lexer bugs that origami's own
//! self-check would share.

use crate::corpus::{files, manifest};
use crate::root;
use origami_core::{LanguageHint, Options, format};
use serde::Deserialize;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Deserialize)]
struct Lexed {
    path: String,
    tokens: Vec<Tok>,
}

#[derive(Deserialize, PartialEq)]
struct Tok {
    #[serde(rename = "type")]
    kind: String,
    text: String,
}

fn normalize(t: &Tok) -> (String, String) {
    let literal = matches!(
        t.kind.as_str(),
        "String"
            | "StringTemplate"
            | "StringTemplateBegin"
            | "StringTemplateMiddle"
            | "StringTemplateEnd"
            | "Comment"
    );
    let text = if literal {
        t.text.clone()
    } else {
        t.text.to_ascii_uppercase()
    };
    (t.kind.clone(), text)
}

fn lex_all(paths: &[String]) -> Result<Vec<Lexed>, String> {
    let script = root().join("tools/abaplint-oracle/lex.mjs");
    let mut child = Command::new("node")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start node: {e}"))?;
    {
        let mut stdin = child.stdin.take().unwrap();
        for p in paths {
            writeln!(stdin, "{p}").map_err(|e| e.to_string())?;
        }
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(
            "abaplint oracle failed (did you run `npm ci` in tools/abaplint-oracle?)".into(),
        );
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).map_err(|e| e.to_string()))
        .collect()
}

pub fn run() -> Result<(), String> {
    let m = manifest()?;
    let tmp = root().join("target/oracle");
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let opts = Options::default();
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    for r in &m.repo {
        for (rel, path) in files(r) {
            if !rel.ends_with(".abap") {
                continue;
            }
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            let res = format(
                &bytes,
                &LanguageHint::Path(path.to_string_lossy().into_owned()),
                &opts,
            );
            let Some(out) = res.output else { continue };
            let out_path = tmp.join(format!("{}__{}", r.name, rel.replace('/', "__")));
            std::fs::write(&out_path, out).map_err(|e| e.to_string())?;
            inputs.push(path.to_string_lossy().into_owned());
            outputs.push(out_path.to_string_lossy().into_owned());
        }
    }
    let a = lex_all(&inputs)?;
    let b = lex_all(&outputs)?;
    let mut mismatches = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        let xa: Vec<_> = x.tokens.iter().map(normalize).collect();
        let yb: Vec<_> = y.tokens.iter().map(normalize).collect();
        if xa != yb {
            mismatches += 1;
            eprintln!("MISMATCH {} vs {}", x.path, y.path);
        }
    }
    println!(
        "oracle: {} files compared, {mismatches} mismatches",
        a.len()
    );
    if mismatches > 0 {
        Err(format!("{mismatches} oracle mismatch(es)"))
    } else {
        Ok(())
    }
}

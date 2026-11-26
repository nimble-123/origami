//! `oracle`: runs the pinned abaplint lexer (tools/abaplint-oracle) on the
//! input and the output of every ABAP corpus file and compares the token
//! sequences: type plus text, with words compared case-insensitively outside
//! strings, templates and comments. This catches lexer bugs that origami's own
//! self-check would share.
//!
//! Both sides are brought into one form for the two changes the self-check
//! allows (docs/INVARIANTS.md, section 3): a period or comma that follows
//! comments stands in front of them (ABAP-SPACE-001), an ABAP Doc tag
//! line has one blank after the tag and one before `|` (ABAP-DOC-001), and
//! a `"` comment that continues with a letter or digit has one space after
//! the `"` (ABAP-COMMENT-003).

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
    let text = if t.kind == "Comment" {
        abap_doc_line(&t.text)
            .or_else(|| spaced_comment(&t.text))
            .unwrap_or_else(|| t.text.clone())
    } else if literal {
        t.text.clone()
    } else {
        t.text.to_ascii_uppercase()
    };
    (t.kind.clone(), text)
}

/// ABAP-COMMENT-003: `"text` as `" text`; `None` for other comments.
fn spaced_comment(s: &str) -> Option<String> {
    let rest = s.strip_prefix('"')?;
    rest.chars()
        .next()
        .filter(|c| c.is_alphanumeric())
        .map(|_| format!("\" {rest}"))
}

/// `"! @parameter name | text` (or `@raising`, `@exception`) with one blank
/// after the tag and one before `|`; `None` for any other comment.
fn abap_doc_line(s: &str) -> Option<String> {
    let blank = |c: char| c == ' ' || c == '\t';
    let rest = s.strip_prefix("\"!")?.trim_start_matches(blank);
    let tag = ["@parameter", "@raising", "@exception"]
        .into_iter()
        .find(|t| {
            rest.get(..t.len())
                .is_some_and(|r| r.eq_ignore_ascii_case(t))
        })?;
    let head = &s[..s.len() - rest.len() + tag.len()];
    let after = &rest[tag.len()..];
    let name_part = after.trim_start_matches(blank);
    if name_part.len() == after.len() {
        return None;
    }
    let end = name_part.find(|c: char| blank(c) || c == '|')?;
    let bar = name_part[end..].trim_start_matches(blank);
    if end == 0 || !bar.starts_with('|') {
        return None;
    }
    Some(format!("{head} {} {bar}", &name_part[..end]))
}

/// The normalized tokens, with every period and comma moved in front of the
/// comments directly before it.
fn canonical(tokens: &[Tok]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::with_capacity(tokens.len());
    for t in tokens {
        let tok = normalize(t);
        if tok.0 == "Punctuation" && matches!(tok.1.as_str(), "." | ",") {
            let at = out.len() - out.iter().rev().take_while(|c| c.0 == "Comment").count();
            out.insert(at, tok);
        } else {
            out.push(tok);
        }
    }
    out
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
    if inputs.is_empty() {
        return Err("no ABAP corpus files to compare (run `cargo xtask corpus fetch`)".into());
    }
    let a = lex_all(&inputs)?;
    let b = lex_all(&outputs)?;
    for (side, lexed) in [("inputs", &a), ("outputs", &b)] {
        if lexed.len() != inputs.len() {
            return Err(format!(
                "abaplint oracle returned {} results for {} {side}",
                lexed.len(),
                inputs.len()
            ));
        }
    }
    let mut mismatches = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        let xa = canonical(&x.tokens);
        let yb = canonical(&y.tokens);
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

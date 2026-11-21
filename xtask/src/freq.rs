//! `freq`: counts ABAP statements by their leading keywords across the corpus.
//! The result is the M1 implementation order for statement parsers.

use crate::corpus::{files, manifest};
use std::collections::HashMap;

pub fn run(top: usize) -> Result<(), String> {
    let m = manifest()?;
    let mut counts: HashMap<String, u64> = HashMap::new();
    let mut total = 0u64;
    for r in &m.repo {
        for (_, path) in files(r) {
            if !path.to_string_lossy().ends_with(".abap") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            for key in statement_keys(&text) {
                *counts.entry(key).or_default() += 1;
                total += 1;
            }
        }
    }
    let mut v: Vec<_> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!("{total} statements");
    let mut cumulative = 0u64;
    for (k, n) in v.into_iter().take(top) {
        cumulative += n;
        println!(
            "{n:>8} {:>6.1}% {:>6.1}%  {k}",
            n as f64 * 100.0 / total as f64,
            cumulative as f64 * 100.0 / total as f64
        );
    }
    Ok(())
}

/// Leading keyword(s) of each statement, from a rough split that ignores
/// comments and literals. Good enough for ranking.
fn statement_keys(text: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut stmt = String::new();
    for line in text.lines() {
        if line.starts_with('*') {
            continue;
        }
        let mut in_lit: Option<char> = None;
        for c in line.chars() {
            match in_lit {
                Some(q) if c == q => in_lit = None,
                Some(_) => {}
                None if c == '"' => break,
                None if c == '\'' || c == '`' || c == '|' => {
                    in_lit = Some(c);
                    stmt.push('_');
                }
                None if c == '.' => {
                    if let Some(k) = key(&stmt) {
                        keys.push(k);
                    }
                    stmt.clear();
                }
                None => stmt.push(c),
            }
        }
        stmt.push(' ');
    }
    keys
}

fn key(stmt: &str) -> Option<String> {
    let words: Vec<String> = stmt
        .split(|c: char| c.is_whitespace() || c == ':' || c == '(')
        .filter(|w| !w.is_empty())
        .map(|w| w.to_ascii_uppercase())
        .collect();
    let first = words.first()?.clone();
    // Assignments and calls: classify by shape.
    if words
        .get(1)
        .is_some_and(|w| w == "=" || w == "?=" || w == "+=" || w == "-=" || w == "&&=")
    {
        return Some("<assignment>".into());
    }
    if first.contains("->")
        || first.contains("=>")
        || stmt.trim_start().contains("( ") && words.len() <= 3
    {
        return Some("<method call>".into());
    }
    let two = |w: &str| {
        words
            .get(1)
            .map(|x| format!("{first} {x}"))
            .unwrap_or_else(|| w.to_string())
    };
    Some(match first.as_str() {
        "CLASS" | "INTERFACE" => {
            if words.iter().any(|w| w == "DEFINITION") {
                format!("{first} … DEFINITION")
            } else if words.iter().any(|w| w == "IMPLEMENTATION") {
                format!("{first} … IMPLEMENTATION")
            } else {
                first
            }
        }
        "READ" | "MODIFY" | "DELETE" | "INSERT" | "CALL" | "RAISE" | "LOOP" | "SELECT" | "AT" => {
            two(&first)
        }
        _ => first,
    })
}

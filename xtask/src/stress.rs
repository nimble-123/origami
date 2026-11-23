//! `stress [iterations] [seed]`: mutation testing on stable Rust.
//!
//! Takes corpus files, applies random mutations (delete, insert or duplicate
//! bytes and lines, insert ABAP/CDS punctuation, comment and quote
//! characters), formats the result and checks:
//! * no X001 (self-check failed) and no X002 (panic);
//! * when formatting succeeds, formatting the output again gives the same
//!   output (idempotency).
//!
//! Each input is formatted at a random combination of line width, indent
//! width and keyword case from `idempotency::options`.
//!
//! Failing inputs are written to target/stress/ for reproduction.

use crate::corpus::{files, manifest};
use crate::root;
use origami_core::{Code, LanguageHint, format};

/// xorshift64*: deterministic, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

const SNIPPETS: &[&[u8]] = &[
    b".",
    b",",
    b":",
    b"(",
    b")",
    b"[",
    b"]",
    b"{",
    b"}",
    b"'",
    b"`",
    b"|",
    b"\"",
    b"*",
    b"\n",
    b" ",
    b"##NO_TEXT",
    b"@",
    b"->",
    b"=>",
    b"~",
    b"//",
    b"--",
    b"/*",
    b"*/",
    b";",
    b"\n*",
    b"IF ",
    b"ENDIF.",
    b"METHOD m.",
    b"ENDMETHOD.",
    b"VALUE #( ",
    b" )",
    b"define ",
    b"key ",
    b"\t",
    b"\r\n",
    b"\xc3\xa4",
];

fn mutate(rng: &mut Rng, src: &[u8]) -> Vec<u8> {
    let mut out = src.to_vec();
    let n = 1 + rng.below(4);
    for _ in 0..n {
        let len = out.len();
        match rng.below(6) {
            0 if len > 0 => {
                let at = rng.below(len);
                let cut = 1 + rng.below(8.min(len - at));
                out.drain(at..at + cut);
            }
            1 => {
                let at = rng.below(len + 1);
                let s = SNIPPETS[rng.below(SNIPPETS.len())];
                out.splice(at..at, s.iter().copied());
            }
            2 if len > 0 => {
                let a = rng.below(len);
                let b = (a + 1 + rng.below(40)).min(len);
                let piece = out[a..b].to_vec();
                let at = rng.below(len + 1);
                out.splice(at..at, piece);
            }
            3 => {
                // Swap two lines.
                let text = String::from_utf8_lossy(&out).into_owned();
                let mut lines: Vec<&str> = text.split('\n').collect();
                if lines.len() > 2 {
                    let a = rng.below(lines.len());
                    let b = rng.below(lines.len());
                    lines.swap(a, b);
                    out = lines.join("\n").into_bytes();
                }
            }
            4 if len > 0 => {
                // Re-indent a random line.
                let at = rng.below(len);
                let line_start = out[..at]
                    .iter()
                    .rposition(|&c| c == b'\n')
                    .map(|p| p + 1)
                    .unwrap_or(0);
                let spaces = rng.below(12);
                out.splice(line_start..line_start, std::iter::repeat_n(b' ', spaces));
            }
            _ => {
                // Join two lines.
                if let Some(p) = out.iter().position(|&c| c == b'\n') {
                    let skip = rng.below(len.max(1));
                    let p = out[skip.min(len)..]
                        .iter()
                        .position(|&c| c == b'\n')
                        .map(|q| q + skip)
                        .unwrap_or(p);
                    if p < out.len() {
                        out[p] = b' ';
                    }
                }
            }
        }
    }
    out
}

pub fn run(args: &[&str]) -> Result<(), String> {
    let iterations: usize = args.first().and_then(|a| a.parse().ok()).unwrap_or(5_000);
    let seed: u64 = args
        .get(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(0x5EED_0F0E_1C0D_E5A7);
    let m = manifest()?;
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    for r in &m.repo {
        for (_, path) in files(r) {
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            if bytes.len() < 40_000 {
                inputs.push((path.to_string_lossy().into_owned(), bytes));
            }
        }
    }
    if inputs.is_empty() {
        return Err("no corpus files — run `cargo xtask corpus fetch`".into());
    }
    let out_dir = root().join("target/stress");
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    // Each input gets a random line width, indent and keyword case.
    let settings = crate::idempotency::options();
    let mut rng = Rng(seed | 1);
    let mut failures = 0;
    let mut formatted = 0;
    let mut errors = 0;
    for it in 0..iterations {
        let (name, src) = &inputs[rng.below(inputs.len())];
        let mutated = mutate(&mut rng, src);
        let hint = LanguageHint::Path(name.clone());
        let opts = &settings[rng.below(settings.len())];
        let r = format(&mutated, &hint, opts);
        let mut problem = None;
        if r.has(Code::X001) || r.has(Code::X002) {
            let d = r
                .diagnostics
                .iter()
                .find(|d| matches!(d.code, Code::X001 | Code::X002))
                .unwrap();
            problem = Some(format!("{} {}", d.code.as_str(), d.message));
        } else if let Some(out) = &r.output {
            formatted += 1;
            let again = format(out, &hint, opts);
            if again.output.as_deref() != Some(&out[..]) {
                problem = Some("not idempotent".into());
            }
        } else {
            errors += 1;
        }
        if let Some(p) = problem {
            failures += 1;
            let ext = name
                .rsplit('/')
                .next()
                .unwrap_or("x")
                .split_once('.')
                .map(|(_, e)| e)
                .unwrap_or("abap");
            let file = out_dir.join(format!("case{it}.{ext}"));
            std::fs::write(&file, &mutated).map_err(|e| e.to_string())?;
            if failures <= 20 {
                eprintln!(
                    "FAIL {} ({p}, {}) from {name}",
                    file.display(),
                    crate::idempotency::label(opts)
                );
            }
        }
    }
    println!(
        "stress: {iterations} mutated inputs (seed {seed}): {formatted} formatted, {errors} rejected with E-codes, {failures} failures"
    );
    if failures > 0 {
        Err(format!("{failures} stress failure(s)"))
    } else {
        Ok(())
    }
}

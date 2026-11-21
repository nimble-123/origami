//! `bench-check`: compares benchmark results with the G4 targets, with a 20%
//! tolerance.
//!
//! * criterion medians (`target/criterion/median.*/new/estimates.json`) are
//!   the in-process p50 per language: target 10 ms;
//! * each hyperfine export given on the command line is checked: files named
//!   `single-*.json` against the end-to-end p95 target of 50 ms, and
//!   `thousand.json` against the 1,000-file target of 1 s (mean).

use crate::root;
use serde_json::Value;

const TOLERANCE: f64 = 1.2;

pub fn run(hyperfine: &[&str]) -> Result<(), String> {
    let mut failures = Vec::new();
    let mut checked = 0;
    let crit = root().join("target/criterion");
    if let Ok(entries) = std::fs::read_dir(&crit) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.starts_with("median") {
                continue;
            }
            let est = e.path().join("new/estimates.json");
            let Ok(text) = std::fs::read_to_string(&est) else {
                continue;
            };
            let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            let ns = v["median"]["point_estimate"].as_f64().unwrap_or(f64::MAX);
            let ms = ns / 1e6;
            checked += 1;
            println!("{name}: p50 {ms:.3} ms (target 10 ms)");
            if ms > 10.0 * TOLERANCE {
                failures.push(format!("{name}: {ms:.3} ms > 12 ms"));
            }
        }
    }
    for f in hyperfine {
        let text = std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?;
        let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        for r in v["results"].as_array().into_iter().flatten() {
            let mut times: Vec<f64> = r["times"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_f64)
                .collect();
            times.sort_by(|a, b| a.partial_cmp(b).unwrap());
            if times.is_empty() {
                continue;
            }
            checked += 1;
            let cmd = r["command"].as_str().unwrap_or("?");
            if f.contains("thousand") {
                let mean = r["mean"].as_f64().unwrap_or(f64::MAX);
                println!("{cmd}: mean {:.1} ms (target 1000 ms)", mean * 1e3);
                if mean > 1.0 * TOLERANCE {
                    failures.push(format!("{cmd}: {:.1} ms > 1200 ms", mean * 1e3));
                }
            } else {
                let idx = ((times.len() as f64) * 0.95).ceil() as usize - 1;
                let p95 = times[idx.min(times.len() - 1)];
                println!("{cmd}: p95 {:.1} ms (target 50 ms)", p95 * 1e3);
                if p95 > 0.050 * TOLERANCE {
                    failures.push(format!("{cmd}: p95 {:.1} ms > 60 ms", p95 * 1e3));
                }
            }
        }
    }
    if checked == 0 {
        return Err("no benchmark results found (run `cargo bench -p origami_core` and scripts/bench-cli.sh)".into());
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n"))
    }
}

//! `cargo xtask bench-matrix`: measures origami in the ways it is used and
//! writes `docs/benchmark-data.json` and `docs/benchmark.md`.
//!
//! Scenarios (all on the test corpus, `cargo xtask corpus fetch` first):
//!
//! * per file, in process (a warm editor server or MCP host): the latency
//!   distribution over every corpus file, for `format` and for `explain`
//!   (the work behind `--review`);
//! * one file, cold: a new `origami` process per call, as format-on-save
//!   without a server, a pre-commit hook or an agent shelling out;
//! * MCP: one `origami --mcp` process, one `format` request per file;
//! * WebAssembly in Node (the website playground), if Node is installed;
//! * the whole corpus from the command line with 1 to 10 threads, plus the
//!   human, agent and review outputs;
//! * ABAP Cleaner (default profile, per file warm, and cold JVM start), if
//!   `ABAP_CLEANER_PLUGINS` points to the plugins folder of its macOS app.
//!
//! `--report` only rewrites docs/benchmark.md from the data file.
//!
//! Needs `cargo build --release -p abap-origami` (and for WebAssembly
//! `cargo build --release -p origami_wasm --target wasm32-unknown-unknown`).

use crate::corpus::{files, manifest};
use crate::root;
use origami_core::{LanguageHint, Options, explain, format};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

/// Nearest-rank percentile of sorted values.
pub(crate) fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let rank = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

/// p50, p95, p99, max and mean (milliseconds) of samples in milliseconds.
pub(crate) fn summary(mut v: Vec<f64>) -> Value {
    v.sort_by(|a, b| a.total_cmp(b));
    let mean = v.iter().sum::<f64>() / v.len().max(1) as f64;
    json!({
        "n": v.len(),
        "p50": round(pct(&v, 50.0)),
        "p95": round(pct(&v, 95.0)),
        "p99": round(pct(&v, 99.0)),
        "max": round(*v.last().unwrap_or(&f64::NAN)),
        "mean": round(mean),
    })
}

pub(crate) fn round(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

pub(crate) fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

pub(crate) struct Source {
    pub(crate) path: PathBuf,
    pub(crate) bytes: Vec<u8>,
    pub(crate) lines: usize,
    pub(crate) abap: bool,
}

pub(crate) fn sources() -> Result<Vec<Source>, String> {
    let m = manifest()?;
    let mut out = Vec::new();
    for r in &m.repo {
        for (_, path) in files(r) {
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            let lines = bytes.iter().filter(|&&b| b == b'\n').count();
            let abap = path.to_string_lossy().ends_with(".abap");
            out.push(Source {
                path,
                bytes,
                lines,
                abap,
            });
        }
    }
    if out.is_empty() {
        return Err("no corpus files; run `cargo xtask corpus fetch`".into());
    }
    Ok(out)
}

pub(crate) fn binary() -> Result<PathBuf, String> {
    let b = root().join("target/release/origami");
    if b.is_file() {
        Ok(b)
    } else {
        Err("target/release/origami missing; run `cargo build --release -p abap-origami`".into())
    }
}

/// `format` in process, per source: the median of three runs, in ms.
pub(crate) fn format_times(src: &[Source]) -> Vec<f64> {
    let opts = Options::default();
    src.iter()
        .map(|s| {
            let hint = LanguageHint::Path(s.path.to_string_lossy().into_owned());
            let mut runs = [0.0; 3];
            for r in &mut runs {
                let t = Instant::now();
                std::hint::black_box(format(&s.bytes, &hint, &opts));
                *r = ms(t);
            }
            runs.sort_by(|a, b| a.total_cmp(b));
            runs[1]
        })
        .collect()
}

/// Per file in process: the median of three runs per file.
fn in_process(src: &[Source]) -> (Value, Value, Value) {
    let opts = Options::default();
    let f = format_times(src);
    let per_line = f
        .iter()
        .zip(src)
        .map(|(t, s)| t * 1000.0 / s.lines.max(1) as f64)
        .collect();
    let mut e = Vec::new();
    for s in src {
        let hint = LanguageHint::Path(s.path.to_string_lossy().into_owned());
        let t = Instant::now();
        std::hint::black_box(explain(&s.bytes, &hint, &opts, &[]));
        e.push(ms(t));
    }
    (summary(f), summary(e), summary(per_line))
}

/// Representative files: the median-sized, the 95th-percentile-sized and the
/// largest ABAP file.
fn representative(src: &[Source]) -> Vec<&Source> {
    let mut abap: Vec<&Source> = src.iter().filter(|s| s.abap).collect();
    abap.sort_by_key(|s| s.lines);
    let at = |p: f64| abap[((p / 100.0) * (abap.len() - 1) as f64).round() as usize];
    vec![at(50.0), at(95.0), abap[abap.len() - 1]]
}

/// Wall times in ms of `runs` calls of `cmd` (rebuilt for each call), after
/// `warmups` unmeasured calls.
pub(crate) fn wall_times(
    cmd: impl Fn() -> Command,
    warmups: usize,
    runs: usize,
) -> Result<Vec<f64>, String> {
    let mut v = Vec::new();
    for i in 0..warmups + runs {
        let t = Instant::now();
        cmd()
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| e.to_string())?;
        if i >= warmups {
            v.push(ms(t));
        }
    }
    Ok(v)
}

fn cold(bin: &Path, files: &[&Source], runs: usize) -> Result<Value, String> {
    let mut rows = Vec::new();
    for s in files {
        let v = wall_times(
            || {
                let mut c = Command::new(bin);
                c.args(["--no-config", "--check", "--diagnostics", "json"])
                    .arg(&s.path);
                c
            },
            2,
            runs,
        )?;
        rows.push(json!({ "lines": s.lines, "time": summary(v) }));
    }
    Ok(Value::Array(rows))
}

fn mcp(bin: &Path, src: &[Source], step: usize) -> Result<Value, String> {
    let mut child = Command::new(bin)
        .arg("--mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let mut stdout = BufReader::new(child.stdout.take().ok_or("no stdout")?);
    let mut line = String::new();
    let mut send = |msg: Value, wait: bool| -> Result<f64, String> {
        let t = Instant::now();
        writeln!(stdin, "{msg}").map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())?;
        if wait {
            line.clear();
            stdout.read_line(&mut line).map_err(|e| e.to_string())?;
        }
        Ok(ms(t))
    };
    send(
        json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{}}}),
        true,
    )?;
    send(
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        false,
    )?;
    let mut v = Vec::new();
    for (k, s) in src.iter().step_by(step).enumerate() {
        let name = s.path.file_name().unwrap_or_default().to_string_lossy();
        let msg = json!({"jsonrpc":"2.0","id":k + 1,"method":"tools/call","params":{"name":"format","arguments":{
            "source": String::from_utf8_lossy(&s.bytes), "filename": name}}});
        v.push(send(msg, true)?);
    }
    let _ = child.kill();
    Ok(summary(v))
}

fn wasm(src: &[Source], step: usize) -> Option<Value> {
    let module = root().join("target/wasm32-unknown-unknown/release/origami_wasm.wasm");
    if !module.is_file() || Command::new("node").arg("--version").output().is_err() {
        return None;
    }
    let list = root().join("target/bench/wasm-files.txt");
    let picked: Vec<String> = src
        .iter()
        .step_by(step)
        .map(|s| format!("{}\t{}", if s.abap { 0 } else { 1 }, s.path.display()))
        .collect();
    std::fs::write(&list, picked.join("\n")).ok()?;
    let script = r#"
const fs = require('fs');
const [module, list] = process.argv.slice(1);
(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(module), {});
  const w = instance.exports;
  const times = [];
  for (const row of fs.readFileSync(list, 'utf8').split('\n')) {
    const [cds, path] = row.split('\t');
    const bytes = fs.readFileSync(path);
    const t = performance.now();
    const p = w.alloc(bytes.length);
    new Uint8Array(w.memory.buffer, p, bytes.length).set(bytes);
    const r = w.review(p, bytes.length, Number(cds), 120, 2, 0);
    const n = w.result_len();
    JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer, r, n)));
    w.dealloc(r, n);
    times.push(performance.now() - t);
  }
  console.log(JSON.stringify(times));
})();
"#;
    let out = Command::new("node")
        .args(["-e", script])
        .arg(&module)
        .arg(&list)
        .output()
        .ok()?;
    let times: Vec<f64> = serde_json::from_slice(&out.stdout).ok()?;
    Some(summary(times))
}

/// Wall time of a whole-corpus run; the first run warms the file cache.
fn project(bin: &Path, args: &[&str], runs: usize) -> Result<Value, String> {
    let v = wall_times(
        || {
            let mut c = Command::new(bin);
            c.args(["--no-config"])
                .args(args)
                .arg(root().join("corpus/repos"))
                .env_remove("CLAUDECODE");
            c
        },
        1,
        runs,
    )?;
    Ok(summary(v))
}

/// Peak resident memory in MB from the stderr of macOS `/usr/bin/time -l`.
pub(crate) fn rss_from_time(stderr: &str) -> Option<f64> {
    let bytes: f64 = stderr
        .lines()
        .find(|l| l.contains("maximum resident set size"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    Some(round(bytes / 1_048_576.0))
}

/// Peak resident memory in MB of one run of `cmd` under `/usr/bin/time -l`.
pub(crate) fn peak_rss(cmd: &Command) -> Option<f64> {
    let out = Command::new("/usr/bin/time")
        .arg("-l")
        .arg(cmd.get_program())
        .args(cmd.get_args())
        .stdout(Stdio::null())
        .output()
        .ok()?;
    rss_from_time(&String::from_utf8_lossy(&out.stderr))
}

/// Compiles the ABAP Cleaner harness (Bench.java with Compare.java, which
/// holds the layout-only profile) into target/bench/acb and returns the
/// class path.
pub(crate) fn compile_abap_cleaner_bench(plugins: &str) -> Result<String, String> {
    let classes = root().join("target/bench/acb");
    let status = Command::new("javac")
        .args(["-cp", &format!("{plugins}/*"), "-d"])
        .arg(&classes)
        .arg(root().join("scripts/abap-cleaner-bench/Bench.java"))
        .arg(root().join("scripts/abap-cleaner-compare/Compare.java"))
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("javac failed".into());
    }
    Ok(format!("{}:{plugins}/*", classes.display()))
}

fn abap_cleaner(src: &[Source], median: &Source) -> Result<Option<Value>, String> {
    let Ok(plugins) = std::env::var("ABAP_CLEANER_PLUGINS") else {
        return Ok(None);
    };
    let cp = compile_abap_cleaner_bench(&plugins)?;
    let list = root().join("target/bench/abap-files.txt");
    let abap: Vec<String> = src
        .iter()
        .filter(|s| s.abap)
        .map(|s| s.path.display().to_string())
        .collect();
    std::fs::write(&list, abap.join("\n")).map_err(|e| e.to_string())?;
    let per_file = root().join("target/bench/abap-cleaner-per-file.txt");
    let t = Instant::now();
    Command::new("java")
        .args(["-Xmx2g", "-cp", &cp, "Bench", "default"])
        .arg(&list)
        .arg("3")
        .arg(&per_file)
        .stdout(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    let total_three_passes = ms(t);
    let times: Vec<f64> = std::fs::read_to_string(&per_file)
        .map_err(|e| e.to_string())?
        .lines()
        .filter_map(|l| l.split('\t').next()?.parse::<f64>().ok())
        .map(|us| us / 1000.0)
        .collect();
    let corpus_ms: f64 = times.iter().sum();
    let one = root().join("target/bench/abap-one.txt");
    std::fs::write(&one, median.path.display().to_string()).map_err(|e| e.to_string())?;
    let mut cold = Vec::new();
    for _ in 0..10 {
        let t = Instant::now();
        Command::new("java")
            .args(["-cp", &cp, "Bench", "default"])
            .arg(&one)
            .arg("1")
            .stdout(Stdio::null())
            .status()
            .map_err(|e| e.to_string())?;
        cold.push(ms(t));
    }
    Ok(Some(json!({
        "version": "1.29.2",
        "profile": "default",
        "per_file_warm": summary(times),
        "corpus_warm_ms": round(corpus_ms),
        "three_passes_wall_ms": round(total_three_passes),
        "one_file_cold": summary(cold),
    })))
}

pub(crate) fn machine() -> Value {
    let sysctl = |k: &str| {
        Command::new("sysctl")
            .args(["-n", k])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    };
    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    json!({
        "cpu": sysctl("machdep.cpu.brand_string"),
        "cores": sysctl("hw.ncpu"),
        "memory_gb": sysctl("hw.memsize").parse::<f64>().map(|b| b / 1_073_741_824.0).unwrap_or(0.0),
        "os": std::env::consts::OS,
        "commit": commit,
    })
}

/// Rewrites docs/benchmark.md from docs/benchmark-data.json without
/// measuring (`cargo xtask bench-matrix --report`).
pub fn report_only() -> Result<(), String> {
    let text = std::fs::read_to_string(root().join("docs/benchmark-data.json"))
        .map_err(|e| e.to_string())?;
    let data: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    std::fs::write(root().join("docs/benchmark.md"), report(&data)).map_err(|e| e.to_string())
}

pub fn run() -> Result<(), String> {
    std::fs::create_dir_all(root().join("target/bench")).map_err(|e| e.to_string())?;
    let bin = binary()?;
    let src = sources()?;
    let abap_files = src.iter().filter(|s| s.abap).count();
    let lines: usize = src.iter().map(|s| s.lines).sum();
    eprintln!(
        "bench-matrix: {} files ({abap_files} ABAP), {lines} lines",
        src.len()
    );

    eprintln!("  in process …");
    let (fmt, exp, per_line) = in_process(&src);
    let reps = representative(&src);
    eprintln!("  cold CLI …");
    let cold_cli = cold(&bin, &reps, 30)?;
    eprintln!("  MCP …");
    let mcp_v = mcp(&bin, &src, 4)?;
    eprintln!("  WebAssembly …");
    let wasm_v = wasm(&src, 4);
    eprintln!("  whole corpus …");
    let mut threads = Vec::new();
    for j in [1, 2, 4, 8, 10] {
        let js = j.to_string();
        threads.push(json!({ "threads": j, "time": project(&bin, &["--check", "-j", &js], 5)? }));
    }
    let outputs = json!({
        "human": project(&bin, &["--check", "--diagnostics", "human"], 5)?,
        "agent": project(&bin, &["--check", "--diagnostics", "agent"], 5)?,
        "json": project(&bin, &["--check", "--diagnostics", "json"], 5)?,
        "review": project(&bin, &["--review"], 5)?,
    });
    let mut rss_cmd = Command::new(&bin);
    rss_cmd
        .args(["--no-config", "--check", "-j", "10"])
        .arg(root().join("corpus/repos"));
    let rss = peak_rss(&rss_cmd);
    eprintln!("  ABAP Cleaner …");
    let acb = abap_cleaner(&src, reps[0])?;

    let data = json!({
        "date": today(),
        "machine": machine(),
        "corpus": { "files": src.len(), "abap_files": abap_files, "lines": lines },
        "per_file_in_process": { "format": fmt, "explain": exp, "format_us_per_line": per_line },
        "one_file_cold_cli": cold_cli,
        "mcp_per_request": mcp_v,
        "wasm_per_file": wasm_v,
        "corpus_by_threads": threads,
        "corpus_by_output": outputs,
        "peak_rss_mb": rss,
        "abap_cleaner": acb,
    });
    let path = root().join("docs/benchmark-data.json");
    std::fs::write(&path, serde_json::to_string_pretty(&data).unwrap() + "\n")
        .map_err(|e| e.to_string())?;
    std::fs::write(root().join("docs/benchmark.md"), report(&data)).map_err(|e| e.to_string())?;
    eprintln!("bench-matrix: wrote docs/benchmark-data.json and docs/benchmark.md");
    Ok(())
}

pub(crate) fn today() -> String {
    Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

pub(crate) fn f(v: &Value) -> String {
    match v.as_f64() {
        Some(x) if x >= 100.0 => format!("{x:.0}"),
        Some(x) if x >= 10.0 => format!("{x:.1}"),
        Some(x) => format!("{x:.2}"),
        None => "–".into(),
    }
}

fn row(name: &str, s: &Value) -> String {
    format!(
        "| {name} | {} | {} | {} | {} |\n",
        f(&s["p50"]),
        f(&s["p95"]),
        f(&s["p99"]),
        f(&s["max"])
    )
}

/// The Markdown report. Numbers come only from `data`.
fn report(d: &Value) -> String {
    let m = &d["machine"];
    let c = &d["corpus"];
    let mut s = String::new();
    s.push_str("# Performance\n\n");
    s.push_str("<!-- Generated by `cargo xtask bench-matrix` from docs/benchmark-data.json; do not edit by hand. -->\n\n");
    s.push_str(&format!(
        "Measured on {} on {} ({} cores, {:.0} GB), origami at commit `{}`, on the test corpus: {} files ({} ABAP), {} lines. All times in milliseconds. p95 and p99 are the latencies that 95 % and 99 % of the calls stay under; max is the slowest call.\n\n",
        d["date"].as_str().unwrap_or(""),
        m["cpu"].as_str().unwrap_or(""),
        m["cores"].as_str().unwrap_or(""),
        m["memory_gb"].as_f64().unwrap_or(0.0),
        m["commit"].as_str().unwrap_or(""),
        c["files"], c["abap_files"], c["lines"],
    ));
    s.push_str("<picture>\n  <source media=\"(prefers-color-scheme: dark)\" srcset=\"assets/latency-dark.svg\">\n  <img alt=\"Latency per call, p50 and p99, for origami in process, through MCP, in WebAssembly, as a new process per file, and for ABAP Cleaner warm and cold.\" src=\"assets/latency.svg\" width=\"860\">\n</picture>\n\n");
    s.push_str("## One file\n\nHow long one call takes, depending on how origami is used.\n\n| Use | p50 | p95 | p99 | max |\n|---|---:|---:|---:|---:|\n");
    let p = &d["per_file_in_process"];
    s.push_str(&row(
        "In process, `format` (editor server, library)",
        &p["format"],
    ));
    s.push_str(&row(
        "In process, `explain` (the work behind `--review`)",
        &p["explain"],
    ));
    s.push_str(&row(
        "MCP server, one `format` request (AI agent)",
        &d["mcp_per_request"],
    ));
    if !d["wasm_per_file"].is_null() {
        s.push_str(&row(
            "WebAssembly in Node (website playground)",
            &d["wasm_per_file"],
        ));
    }
    if let Value::Array(rows) = &d["one_file_cold_cli"] {
        for (r, label) in rows
            .iter()
            .zip(["median-sized", "95th-percentile-sized", "largest"])
        {
            s.push_str(&row(
                &format!(
                    "New process per file, {label} ABAP file ({} lines)",
                    r["lines"]
                ),
                &r["time"],
            ));
        }
    }
    let a = &d["abap_cleaner"];
    if !a.is_null() {
        s.push_str(&row(
            "ABAP Cleaner, per file, warm JVM (default profile)",
            &a["per_file_warm"],
        ));
        s.push_str(&row(
            "ABAP Cleaner, new JVM per file (median-sized file)",
            &a["one_file_cold"],
        ));
    }
    s.push_str(&format!(
        "\nIn process, origami needs {} µs per line at the median (p99 {} µs). Per-file rows cover every corpus file (in process) or every fourth file (MCP, WebAssembly); the new-process rows are 30 runs each after 2 warm-up runs.\n\n",
        f(&p["format_us_per_line"]["p50"]),
        f(&p["format_us_per_line"]["p99"]),
    ));
    s.push_str("## Whole corpus\n\nOne command over the whole corpus, as in CI or a pre-commit run over a repository: median of 5 runs after one warm-up run.\n\n");
    s.push_str("<picture>\n  <source media=\"(prefers-color-scheme: dark)\" srcset=\"assets/threads-dark.svg\">\n  <img alt=\"Time to check the whole corpus with 1, 2, 4, 8 and 10 threads.\" src=\"assets/threads.svg\" width=\"860\">\n</picture>\n\n");
    s.push_str("| Threads | p50 | max |\n|---:|---:|---:|\n");
    if let Value::Array(rows) = &d["corpus_by_threads"] {
        for r in rows {
            s.push_str(&format!(
                "| {} | {} | {} |\n",
                r["threads"],
                f(&r["time"]["p50"]),
                f(&r["time"]["max"])
            ));
        }
    }
    if m["cpu"].as_str() == Some("Apple M4") {
        s.push_str("\nBeyond four threads the gain is small on this machine: the Apple M4 has four performance cores and six efficiency cores. By default origami uses one thread per core.\n");
    }
    s.push_str("\n| Output (all threads) | p50 | max |\n|---|---:|---:|\n");
    for (k, label) in [
        ("human", "`--check`, human output"),
        ("agent", "`--check --diagnostics agent`"),
        ("json", "`--check --diagnostics json`"),
        ("review", "`--review` (notes on every changed file)"),
    ] {
        let t = &d["corpus_by_output"][k];
        s.push_str(&format!(
            "| {label} | {} | {} |\n",
            f(&t["p50"]),
            f(&t["max"])
        ));
    }
    if let Some(mb) = d["peak_rss_mb"].as_f64() {
        s.push_str(&format!(
            "\nPeak memory of a whole-corpus run with 10 threads: {mb:.0} MB.\n"
        ));
    }
    if !a.is_null() {
        s.push_str(&format!(
            "\n## ABAP Cleaner\n\nABAP Cleaner {} with its {} profile processes the ABAP files of the corpus in {} ms (sum of the per-file times, warm JVM, one thread). It applies about 100 cleanup rules, several of which change code; origami only changes layout, so the numbers compare the time to process the same files, not the same work. How it was run: [benchmark-abap-cleaner.md](benchmark-abap-cleaner.md).\n\nThe comparison with ABAP Cleaner's layout and default profile, cold and warm, for one statement, one method, one file and the whole project (`cargo xtask bench-compare`, all numbers in [benchmark-abap-cleaner.md](benchmark-abap-cleaner.md)):\n\n<picture>\n  <source media=\"(prefers-color-scheme: dark)\" srcset=\"assets/granularity-dark.svg\">\n  <img alt=\"Median time per call for one statement, one method and one file, warm and cold, for origami and ABAP Cleaner with its layout and default profile, on a log scale.\" src=\"assets/granularity.svg\" width=\"860\">\n</picture>\n\n<picture>\n  <source media=\"(prefers-color-scheme: dark)\" srcset=\"assets/benchmark-dark.svg\">\n  <img alt=\"Time to format all ABAP files of the test corpus: ABAP Cleaner with its default and its layout profile, origami on one thread and on one thread per core.\" src=\"assets/benchmark.svg\" width=\"860\">\n</picture>\n",
            a["version"].as_str().unwrap_or(""),
            a["profile"].as_str().unwrap_or(""),
            f(&a["corpus_warm_ms"]),
        ));
    }
    s.push_str("\n## Reproduce\n\n```sh\ncargo xtask corpus fetch\ncargo build --release -p abap-origami\ncargo build --release -p origami_wasm --target wasm32-unknown-unknown   # optional\nABAP_CLEANER_PLUGINS=…/abapcleaner.app/Contents/Eclipse/plugins \\\n  cargo xtask bench-matrix                                            # ABAP Cleaner optional\npython3 docs/diagrams/build.py                                        # charts\n```\n");
    s
}

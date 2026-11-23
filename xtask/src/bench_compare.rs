//! `cargo xtask bench-compare`: origami against ABAP Cleaner, cold and warm,
//! for one statement, one method, one file and the whole project. Writes
//! `docs/benchmark-compare-data.json` and the generated section of
//! `docs/benchmark-abap-cleaner.md`.
//!
//! Inputs, built from the ABAP files of the test corpus:
//!
//! * file: every ABAP file;
//! * method: every k-th `METHOD … ENDMETHOD.` block of the class files
//!   (about 500; AMDP methods, `BY DATABASE …`, are left out);
//! * statement: every k-th complete statement inside those methods (about
//!   500): it starts a line, ends with a period followed by nothing but a
//!   comment, and does not open, continue or close a block (`IF`, `LOOP`,
//!   `CATCH`, `BEGIN OF` …), because such a statement alone is not valid
//!   code for either tool;
//! * project: all ABAP files in one run.
//!
//! Methods and statements are written to target/bench/compare/ as
//! `<n>.prog.abap`, so that both tools read the same bytes.
//!
//! Measurements, strictly one after another:
//!
//! * warm, per input: origami's library `format` (median of 3 runs) and
//!   ABAP Cleaner in one JVM (3 passes, the last one counts), with its
//!   layout-only profile and its default profile;
//! * cold: a new process per call on the median-sized input, 20 runs of the
//!   origami CLI after 2 warm-up runs and 8 runs of a new JVM after one;
//! * project: the origami CLI over all ABAP files with 1 thread and with one
//!   thread per core (first run, then the median of 5 runs), ABAP Cleaner in
//!   one JVM (first pass including the JVM start, third pass).
//!
//! Peak memory comes from one extra run under `/usr/bin/time -l` (for the
//! long ABAP Cleaner runs, from the measured run itself). Before every heavy
//! step the free memory is checked (`vm_stat`); with less than 3 GB the run
//! stops instead of swapping.
//!
//! Run `cargo build --release -p abap-origami -p xtask`, then
//! `target/release/xtask bench-compare` with `ABAP_CLEANER_PLUGINS` set to
//! the plugins folder of ABAP Cleaner's macOS app (`cargo xtask` would
//! measure origami's library without optimizations). `--report` only
//! rewrites the Markdown section from the data file.

use crate::bench_matrix::{
    Source, binary, compile_abap_cleaner_bench, f, format_times, machine, ms, peak_rss, round,
    rss_from_time, sources, summary, today, wall_times,
};
use crate::root;
use origami_core::{LanguageHint, Options, Status, format};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

const SAMPLE: usize = 500;
const ORIGAMI_COLD_RUNS: usize = 20;
const JVM_COLD_RUNS: usize = 8;
const PROJECT_RUNS: usize = 5;
const PROFILES: [&str; 2] = ["layout", "default"];
const MIN_FREE_GB: f64 = 3.0;
const START: &str = "<!-- bench-compare:start -->";
const END: &str = "<!-- bench-compare:end -->";

/// One granularity: its inputs and the file list ABAP Cleaner reads.
struct Level {
    key: &'static str,
    inputs: Vec<Source>,
    list: PathBuf,
}

fn is_comment(line: &str) -> bool {
    line.starts_with('*') || line.trim_start().starts_with('"')
}

fn first_word(text: &str) -> String {
    text.trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .to_ascii_uppercase()
}

/// `METHOD … ENDMETHOD.` blocks of one file, as written (AMDP methods left
/// out).
fn methods(code: &str) -> Vec<String> {
    let lines: Vec<&str> = code.split_inclusive('\n').collect();
    let mut out = Vec::new();
    let mut start = None;
    for (i, line) in lines.iter().enumerate() {
        if is_comment(line) {
            continue;
        }
        match (first_word(line).as_str(), start) {
            ("METHOD", None) => start = Some(i),
            ("ENDMETHOD", Some(s)) => {
                let mut text = lines[s..=i].concat();
                if !text.ends_with('\n') {
                    text.push('\n');
                }
                let head = text.split('.').next().unwrap_or("").to_ascii_uppercase();
                if !head.contains("BY DATABASE") {
                    out.push(text);
                }
                start = None;
            }
            _ => {}
        }
    }
    out
}

/// Statements that are not valid code on their own: they open, continue or
/// close a block.
const BLOCK: &[&str] = &[
    "IF",
    "ELSEIF",
    "ELSE",
    "ENDIF",
    "CASE",
    "WHEN",
    "ENDCASE",
    "DO",
    "ENDDO",
    "WHILE",
    "ENDWHILE",
    "LOOP",
    "ENDLOOP",
    "TRY",
    "CATCH",
    "CLEANUP",
    "ENDTRY",
    "ENDSELECT",
    "AT",
    "ENDAT",
    "PROVIDE",
    "ENDPROVIDE",
    "ON",
    "ENDON",
    "WITH",
    "ENDWITH",
    "EXEC",
    "ENDEXEC",
    "ENDCATCH",
    "TEST-SEAM",
    "END-TEST-SEAM",
    "TEST-INJECTION",
    "END-TEST-INJECTION",
    "DEFINE",
    "END-OF-DEFINITION",
    "ENHANCEMENT-SECTION",
    "END-ENHANCEMENT-SECTION",
    "METHOD",
    "ENDMETHOD",
];

/// True if `BEGIN OF` and `END OF` do not pair up within the statement.
fn unbalanced_structure(text: &str) -> bool {
    let upper = text.to_ascii_uppercase();
    let words: Vec<&str> = upper
        .split(|c: char| c.is_whitespace() || c == ',' || c == ':')
        .filter(|w| !w.is_empty())
        .collect();
    let count = |w: &str| {
        words
            .windows(2)
            .filter(|p| p[0] == w && p[1] == "OF")
            .count()
    };
    count("BEGIN") != count("END")
}

/// Complete statements inside a method, as written: each starts a line and
/// ends with a period followed by nothing but a comment. Comment lines never
/// start a statement; inside a multi-line statement they are kept.
fn statements(method: &str) -> Vec<String> {
    let lines: Vec<&str> = method.split_inclusive('\n').collect();
    let body = &lines[1.min(lines.len())..lines.len().saturating_sub(1)];
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut at_line_start = true;
    for (i, line) in body.iter().enumerate() {
        if is_comment(line) {
            continue;
        }
        let mut quote: Option<char> = None;
        let mut chars = line.char_indices();
        while let Some((pos, c)) = chars.next() {
            if let Some(q) = quote {
                if q == '|' && c == '\\' {
                    chars.next();
                } else if c == q {
                    quote = None;
                }
                continue;
            }
            if c == '"' {
                break;
            }
            if c.is_whitespace() {
                continue;
            }
            if c == '.' {
                if let Some(s) = start {
                    let rest = line[pos + 1..].trim_start();
                    if at_line_start && (rest.is_empty() || rest.starts_with('"')) {
                        let text = body[s..=i].concat();
                        if !BLOCK.contains(&first_word(&text).as_str())
                            && !unbalanced_structure(&text)
                        {
                            out.push(text);
                        }
                    }
                }
                start = None;
                continue;
            }
            if start.is_none() {
                start = Some(i);
                at_line_start = line[..pos].trim().is_empty();
            }
            if matches!(c, '\'' | '`' | '|') {
                quote = Some(c);
            }
        }
    }
    out
}

/// Every k-th element, so that about `n` remain.
fn every_kth<T>(v: Vec<T>, n: usize) -> Vec<T> {
    let k = v.len().div_ceil(n).max(1);
    v.into_iter().step_by(k).collect()
}

/// Writes fragments as `<dir>/<n>.prog.abap` and returns them as sources.
fn write_fragments(dir: &Path, texts: Vec<String>) -> Result<Vec<Source>, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    texts
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            let path = dir.join(format!("{i:04}.prog.abap"));
            std::fs::write(&path, &text).map_err(|e| e.to_string())?;
            Ok(Source {
                lines: text.lines().count(),
                path,
                bytes: text.into_bytes(),
                abap: true,
            })
        })
        .collect()
}

fn write_list(path: &Path, inputs: &[&Source]) -> Result<(), String> {
    let list: Vec<String> = inputs
        .iter()
        .map(|s| s.path.display().to_string())
        .collect();
    std::fs::write(path, list.join("\n") + "\n").map_err(|e| e.to_string())
}

fn levels(files: Vec<Source>) -> Result<Vec<Level>, String> {
    let dir = root().join("target/bench/compare");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    let all_methods: Vec<String> = files
        .iter()
        .filter(|s| s.path.to_string_lossy().contains(".clas."))
        .flat_map(|s| methods(&String::from_utf8_lossy(&s.bytes)))
        .collect();
    let picked = every_kth(all_methods, SAMPLE);
    let all_statements: Vec<String> = picked.iter().flat_map(|m| statements(m)).collect();
    let picked_statements = every_kth(all_statements, SAMPLE);
    let statements = write_fragments(&dir.join("statement"), picked_statements)?;
    let methods = write_fragments(&dir.join("method"), picked)?;
    let mut out = Vec::new();
    for (key, inputs) in [
        ("statement", statements),
        ("method", methods),
        ("file", files),
    ] {
        let list = dir.join(format!("{key}.txt"));
        write_list(&list, &inputs.iter().collect::<Vec<_>>())?;
        out.push(Level { key, inputs, list });
    }
    Ok(out)
}

/// The median-sized input (by lines, then bytes, then path).
fn median(inputs: &[Source]) -> &Source {
    let mut v: Vec<&Source> = inputs.iter().collect();
    v.sort_by(|a, b| (a.lines, a.bytes.len(), &a.path).cmp(&(b.lines, b.bytes.len(), &b.path)));
    v[(v.len() - 1) / 2]
}

fn lines_summary(inputs: &[Source]) -> Value {
    let mut v: Vec<usize> = inputs.iter().map(|s| s.lines).collect();
    v.sort_unstable();
    json!({
        "count": v.len(),
        "lines_total": v.iter().sum::<usize>(),
        "lines_median": v[(v.len() - 1) / 2],
        "lines_p95": v[((v.len() as f64 * 0.95).ceil() as usize).clamp(1, v.len()) - 1],
        "lines_max": v[v.len() - 1],
    })
}

/// Free plus inactive memory in GB (macOS `vm_stat`).
fn available_gb() -> Option<f64> {
    let out = Command::new("vm_stat").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let page: f64 = text
        .split("page size of ")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    let pages = |key: &str| -> Option<f64> {
        text.lines()
            .find(|l| l.starts_with(key))?
            .split(':')
            .nth(1)?
            .trim()
            .trim_end_matches('.')
            .parse()
            .ok()
    };
    Some((pages("Pages free")? + pages("Pages inactive")?) * page / 1_073_741_824.0)
}

fn ensure_memory(step: &str) -> Result<(), String> {
    match available_gb() {
        Some(gb) if gb < MIN_FREE_GB => Err(format!(
            "only {gb:.1} GB of memory available before {step} (free + inactive pages); \
             need {MIN_FREE_GB} GB. Close other programs and run again."
        )),
        _ => Ok(()),
    }
}

fn origami_errors(inputs: &[Source]) -> usize {
    let opts = Options::default();
    inputs
        .iter()
        .filter(|s| {
            let hint = LanguageHint::Path(s.path.to_string_lossy().into_owned());
            format(&s.bytes, &hint, &opts).status == Status::Error
        })
        .count()
}

fn origami_cli(bin: &Path, args: &[&str], paths: &[PathBuf]) -> Command {
    let mut c = Command::new(bin);
    c.current_dir(root())
        .args(["--no-config", "--check"])
        .args(args)
        .args(paths)
        .env_remove("CLAUDECODE");
    c
}

fn origami_cold(bin: &Path, input: &Source) -> Result<Value, String> {
    let paths = [input.path.clone()];
    let v = wall_times(|| origami_cli(bin, &[], &paths), 2, ORIGAMI_COLD_RUNS)?;
    Ok(json!({ "time": summary(v), "peak_rss_mb": peak_rss(&origami_cli(bin, &[], &paths)) }))
}

fn origami_project(bin: &Path, args: &[&str], paths: &[PathBuf]) -> Result<Value, String> {
    ensure_memory("the origami project run")?;
    let v = wall_times(|| origami_cli(bin, args, paths), 0, PROJECT_RUNS + 1)?;
    Ok(json!({
        "cold_ms": round(v[0]),
        "warm": summary(v[1..].to_vec()),
        "peak_rss_mb": peak_rss(&origami_cli(bin, args, paths)),
    }))
}

fn java(cp: &str, xmx: &str, profile: &str, list: &Path, passes: usize) -> Command {
    let mut c = Command::new("java");
    c.arg(format!("-Xmx{xmx}"))
        .args(["-cp", cp, "Bench", profile])
        .arg(list)
        .arg(passes.to_string());
    c
}

/// One ABAP Cleaner JVM run.
struct JvmRun {
    /// Time in ms from the start of the process to each "pass N:" line.
    wall: Vec<f64>,
    /// The pass times in ms that ABAP Cleaner reported.
    reported: Vec<f64>,
    /// Peak memory in MB.
    rss: Option<f64>,
}

/// Runs `cmd` under `/usr/bin/time -l`, timing each "pass N:" line of its
/// output from the start of the process.
fn timed_jvm(cmd: &Command) -> Result<JvmRun, String> {
    let t = Instant::now();
    let mut child = Command::new("/usr/bin/time")
        .arg("-l")
        .arg(cmd.get_program())
        .args(cmd.get_args())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut stderr = child.stderr.take().ok_or("no stderr")?;
    let err_reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    let mut wall = Vec::new();
    let mut reported = Vec::new();
    for line in BufReader::new(child.stdout.take().ok_or("no stdout")?).lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.contains(" pass ") {
            wall.push(ms(t));
            // "abapcleaner <mode> pass <n>: <files> files, <lines> lines, <ms> ms, …"
            if let Some(v) = line
                .split(", ")
                .find(|p| p.ends_with(" ms"))
                .and_then(|p| p.trim_end_matches(" ms").parse::<f64>().ok())
            {
                reported.push(v);
            }
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let err = err_reader.join().unwrap_or_default();
    if !status.success() {
        let tail: Vec<&str> = err.lines().take(5).collect();
        return Err(format!("ABAP Cleaner failed: {}", tail.join(" | ")));
    }
    Ok(JvmRun {
        wall,
        reported,
        rss: rss_from_time(&err),
    })
}

/// Per-input times (µs) and the inputs with errors in the last pass, from
/// Bench's per-file output.
fn per_input(out: &Path) -> Result<(Vec<f64>, Vec<String>), String> {
    let text = std::fs::read_to_string(out).map_err(|e| e.to_string())?;
    let mut times = Vec::new();
    let mut errors = Vec::new();
    for l in text.lines() {
        let cols: Vec<&str> = l.split('\t').collect();
        if let Some(us) = cols.first().and_then(|c| c.parse::<f64>().ok()) {
            times.push(us);
        }
        if let (Some(path), Some(&"1")) = (cols.get(1), cols.get(2)) {
            let p = Path::new(path);
            errors.push(p.strip_prefix(root()).unwrap_or(p).display().to_string());
        }
    }
    Ok((times, errors))
}

fn cleaner_warm(cp: &str, profile: &str, level: &Level) -> Result<Value, String> {
    ensure_memory("an ABAP Cleaner run")?;
    let out = level.list.with_extension(format!("{profile}.out"));
    let mut cmd = java(cp, "1g", profile, &level.list, 3);
    cmd.arg(&out);
    let rss = timed_jvm(&cmd)?.rss;
    let (times, errors) = per_input(&out)?;
    Ok(json!({
        "time_us": summary(times),
        "errors": errors.len(),
        "error_inputs": errors,
        "peak_rss_mb": rss,
    }))
}

fn cleaner_cold(cp: &str, profile: &str, one: &Path) -> Result<Value, String> {
    ensure_memory("an ABAP Cleaner cold run")?;
    let v = wall_times(|| java(cp, "1g", profile, one, 1), 1, JVM_COLD_RUNS)?;
    Ok(json!({ "time": summary(v), "peak_rss_mb": peak_rss(&java(cp, "1g", profile, one, 1)) }))
}

/// One JVM over all ABAP files, three passes. Retries once with a 3 GB heap
/// if 2 GB is not enough.
fn cleaner_project(cp: &str, profile: &str, list: &Path) -> Result<Value, String> {
    let mut notes = Vec::new();
    for xmx in ["2g", "3g"] {
        ensure_memory("the ABAP Cleaner project run")?;
        match timed_jvm(&java(cp, xmx, profile, list, 3)) {
            Ok(r) if r.wall.len() == 3 && r.reported.len() == 3 => {
                return Ok(json!({
                    "xmx": xmx,
                    "cold_ms": round(r.wall[0]),
                    "warm_ms": r.reported[2],
                    "pass_ms": r.reported,
                    "peak_rss_mb": r.rss,
                    "notes": notes,
                }));
            }
            Ok(_) => return Err("ABAP Cleaner did not report three passes".into()),
            Err(e) if xmx == "2g" && e.contains("OutOfMemoryError") => {
                eprintln!(
                    "  ABAP Cleaner {profile}: out of memory with -Xmx2g, retrying with -Xmx3g"
                );
                notes.push(format!("-Xmx2g: {e}"));
            }
            Err(e) => return Err(e),
        }
    }
    Err(format!("ABAP Cleaner {profile}: out of memory with -Xmx3g"))
}

fn command_line(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|o| {
            let text = if o.stdout.is_empty() {
                o.stderr
            } else {
                o.stdout
            };
            String::from_utf8_lossy(&text).trim().to_string()
        })
        .unwrap_or_default()
}

fn cleaner_version(plugins: &str) -> String {
    std::fs::read_dir(plugins)
        .ok()
        .and_then(|d| {
            d.flatten().find_map(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                n.strip_prefix("com.sap.adt.abapcleaner_")
                    .and_then(|v| v.strip_suffix(".jar"))
                    .map(str::to_string)
            })
        })
        .unwrap_or_default()
}

pub fn run() -> Result<(), String> {
    let plugins = std::env::var("ABAP_CLEANER_PLUGINS").map_err(
        |_| "set ABAP_CLEANER_PLUGINS to the plugins folder of ABAP Cleaner's macOS app",
    )?;
    let bin = binary()?;
    std::fs::create_dir_all(root().join("target/bench")).map_err(|e| e.to_string())?;
    let mut files: Vec<Source> = sources()?.into_iter().filter(|s| s.abap).collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let project_paths: Vec<PathBuf> = files
        .iter()
        .map(|s| s.path.strip_prefix(root()).unwrap_or(&s.path).to_path_buf())
        .collect();
    let project_lines: usize = files.iter().map(|s| s.lines).sum();
    let levels = levels(files)?;
    for l in &levels {
        eprintln!(
            "bench-compare: {} inputs of type {}, median {} lines",
            l.inputs.len(),
            l.key,
            median(&l.inputs).lines
        );
    }

    let mut inputs = serde_json::Map::new();
    let mut warm = serde_json::Map::new();
    let mut cold = serde_json::Map::new();
    let mut ones = Vec::new();
    for l in &levels {
        let m = median(&l.inputs);
        let one = l.list.with_extension("median.txt");
        write_list(&one, &[m])?;
        ones.push(one);
        let mut info = lines_summary(&l.inputs);
        info["median_input"] = json!({
            "path": m.path.strip_prefix(root()).unwrap_or(&m.path).display().to_string(),
            "lines": m.lines,
        });
        inputs.insert(l.key.into(), info);
    }
    inputs.insert(
        "project".into(),
        json!({ "count": project_paths.len(), "lines_total": project_lines }),
    );

    eprintln!("  origami, warm, in process …");
    for l in &levels {
        warm.insert(
            l.key.into(),
            json!({ "origami": {
                "time_us": summary(format_times(&l.inputs).iter().map(|t| t * 1000.0).collect()),
                "errors": origami_errors(&l.inputs),
            }}),
        );
    }
    eprintln!("  origami, cold …");
    for l in &levels {
        cold.insert(
            l.key.into(),
            json!({ "origami": origami_cold(&bin, median(&l.inputs))? }),
        );
    }
    eprintln!("  origami, project …");
    let mut project = serde_json::Map::new();
    project.insert(
        "origami_j1".into(),
        origami_project(&bin, &["-j", "1"], &project_paths)?,
    );
    project.insert(
        "origami".into(),
        origami_project(&bin, &[], &project_paths)?,
    );

    let cp = compile_abap_cleaner_bench(&plugins)?;
    for profile in PROFILES {
        for (l, one) in levels.iter().zip(&ones) {
            eprintln!("  ABAP Cleaner {profile}, {} …", l.key);
            warm[l.key][profile] = cleaner_warm(&cp, profile, l)?;
            cold[l.key][profile] = cleaner_cold(&cp, profile, one)?;
        }
        eprintln!("  ABAP Cleaner {profile}, project …");
        let file_list = &levels[2].list;
        project.insert(profile.into(), cleaner_project(&cp, profile, file_list)?);
    }

    let mut m = machine();
    m["os_version"] = json!(command_line("sw_vers", &["-productVersion"]));
    m["jdk"] = json!(
        command_line("java", &["-version"])
            .lines()
            .nth(1)
            .unwrap_or("")
            .to_string()
    );
    m["abap_cleaner"] = json!(cleaner_version(&plugins));
    m["jvm_flags"] =
        json!("-Xmx1g per input and cold, -Xmx2g for the project; default garbage collector");
    let data = json!({
        "date": today(),
        "machine": m,
        "inputs": inputs,
        "warm": warm,
        "cold": cold,
        "project": project,
    });
    let path = root().join("docs/benchmark-compare-data.json");
    std::fs::write(&path, serde_json::to_string_pretty(&data).unwrap() + "\n")
        .map_err(|e| e.to_string())?;
    write_report(&data)?;
    eprintln!(
        "bench-compare: wrote docs/benchmark-compare-data.json and docs/benchmark-abap-cleaner.md"
    );
    Ok(())
}

/// Rewrites the generated section of docs/benchmark-abap-cleaner.md from
/// docs/benchmark-compare-data.json (`cargo xtask bench-compare --report`).
pub fn report_only() -> Result<(), String> {
    let text = std::fs::read_to_string(root().join("docs/benchmark-compare-data.json"))
        .map_err(|e| e.to_string())?;
    write_report(&serde_json::from_str(&text).map_err(|e| e.to_string())?)
}

fn write_report(d: &Value) -> Result<(), String> {
    let path = root().join("docs/benchmark-abap-cleaner.md");
    let doc = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let (Some(a), Some(b)) = (doc.find(START), doc.find(END)) else {
        return Err(format!(
            "{} lacks the markers {START} … {END}",
            path.display()
        ));
    };
    let new = format!("{}{START}\n{}{}", &doc[..a], section(d), &doc[b..]);
    std::fs::write(&path, new).map_err(|e| e.to_string())
}

const LEVELS: [(&str, &str); 3] = [
    ("statement", "One statement"),
    ("method", "One method"),
    ("file", "One file"),
];

fn mb(v: &Value) -> String {
    v.as_f64().map_or("–".into(), |x| format!("{x:.0}"))
}

fn secs(v: &Value) -> String {
    v.as_f64()
        .map_or("–".into(), |x| format!("{:.2} s", x / 1000.0))
}

/// How many times slower than origami, at the median.
fn times(other: &Value, origami: &Value) -> String {
    match (other.as_f64(), origami.as_f64()) {
        (Some(a), Some(b)) if b > 0.0 => {
            let r = a / b;
            if r >= 10.0 {
                format!("{r:.0}×")
            } else {
                format!("{r:.1}×")
            }
        }
        _ => "–".into(),
    }
}

const TOOLS: [(&str, &str); 3] = [
    ("origami", "origami"),
    ("layout", "ABAP Cleaner, layout profile"),
    ("default", "ABAP Cleaner, default profile"),
];

/// The generated Markdown section. Numbers come only from `d`.
fn section(d: &Value) -> String {
    let m = &d["machine"];
    let i = &d["inputs"];
    let mut s = String::new();
    s.push_str("<!-- Generated by `cargo xtask bench-compare` from docs/benchmark-compare-data.json; do not edit by hand. -->\n\n");
    s.push_str(&format!(
        "## Current measurement ({})\n\n{}, {} cores, {:.0} GB RAM, macOS {}, {}, ABAP Cleaner {}, origami at commit `{}`. JVM: {}. All times in milliseconds unless marked.\n\n",
        d["date"].as_str().unwrap_or(""),
        m["cpu"].as_str().unwrap_or(""),
        m["cores"].as_str().unwrap_or(""),
        m["memory_gb"].as_f64().unwrap_or(0.0),
        m["os_version"].as_str().unwrap_or(""),
        m["jdk"].as_str().unwrap_or(""),
        m["abap_cleaner"].as_str().unwrap_or(""),
        m["commit"].as_str().unwrap_or(""),
        m["jvm_flags"].as_str().unwrap_or(""),
    ));
    s.push_str("### Inputs\n\nAll taken from the ABAP files of the test corpus. Methods and statements are cut out of the class files as written (see `xtask/src/bench_compare.rs`) and saved as separate files, so both tools read the same bytes.\n\n| Granularity | Inputs | Median lines | p95 lines | Max lines | Lines of the median input (cold runs) |\n|---|---:|---:|---:|---:|---:|\n");
    for (k, label) in LEVELS {
        let x = &i[k];
        s.push_str(&format!(
            "| {label} | {} | {} | {} | {} | {} |\n",
            x["count"],
            x["lines_median"],
            x["lines_p95"],
            x["lines_max"],
            x["median_input"]["lines"]
        ));
    }
    s.push_str(&format!(
        "| Whole project | {} files, {} lines | – | – | – | – |\n\n",
        i["project"]["count"], i["project"]["lines_total"]
    ));
    s.push_str("### Warm: one call in a running process\n\nAs in an editor or a server that stays open. origami: the library function `format`, median of 3 runs per input. ABAP Cleaner: one JVM, 3 passes over all inputs, the times of the third pass (after the JIT compiler has warmed up). Per input, in **microseconds** (1,000 µs = 1 ms); the last column is the median divided by origami's median.\n\n| Granularity | Tool | p50 | p95 | p99 | max | p50 vs origami |\n|---|---|---:|---:|---:|---:|---:|\n");
    for (k, label) in LEVELS {
        let w = &d["warm"][k];
        for (t, name) in TOOLS {
            let x = &w[t]["time_us"];
            s.push_str(&format!(
                "| {label} | {name} | {} | {} | {} | {} | {} |\n",
                f(&x["p50"]),
                f(&x["p95"]),
                f(&x["p99"]),
                f(&x["max"]),
                if t == "origami" {
                    "–".into()
                } else {
                    times(&x["p50"], &w["origami"]["time_us"]["p50"])
                }
            ));
        }
    }
    let errors: Vec<String> = LEVELS
        .iter()
        .map(|(k, label)| {
            let w = &d["warm"][*k];
            format!(
                "{} ({} origami, {} layout, {} default)",
                label.trim_start_matches("One ").to_owned() + "s",
                w["origami"]["errors"],
                w["layout"]["errors"],
                w["default"]["errors"]
            )
        })
        .collect();
    let failed: Vec<&str> = LEVELS
        .iter()
        .flat_map(|(k, _)| PROFILES.map(|p| &d["warm"][*k][p]["error_inputs"]))
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let screens = if !failed.is_empty() && failed.iter().all(|p| p.contains(".screen_")) {
        " All of ABAP Cleaner's errors are on screen files (`*.screen_NNNN.abap`), which hold dynpro flow logic rather than ABAP statements."
    } else {
        ""
    };
    s.push_str(&format!(
        "\nInputs a tool reported an error on (it then leaves the input unchanged; the time still counts): {}.{screens}\n\n",
        errors.join(", ")
    ));
    s.push_str("### Cold: a new process per call\n\nAs in a pre-commit hook, a format-on-save without a running server, or an agent calling a command: the median-sized input of each granularity, a new process per call. origami: the CLI, `--no-config --check`, 20 runs after 2 warm-up runs. ABAP Cleaner: a new JVM per call through the harness, 8 runs after 1 warm-up run. Peak memory from one more run.\n\n| Granularity | Tool | median (ms) | max (ms) | median vs origami | peak memory (MB) |\n|---|---|---:|---:|---:|---:|\n");
    for (k, label) in LEVELS {
        let c = &d["cold"][k];
        let lines = &i[k]["median_input"]["lines"];
        for (t, name) in TOOLS {
            let x = &c[t];
            s.push_str(&format!(
                "| {label} ({lines} {}) | {name} | {} | {} | {} | {} |\n",
                if lines.as_u64() == Some(1) {
                    "line"
                } else {
                    "lines"
                },
                f(&x["time"]["p50"]),
                f(&x["time"]["max"]),
                if t == "origami" {
                    "–".into()
                } else {
                    times(&x["time"]["p50"], &c["origami"]["time"]["p50"])
                },
                mb(&x["peak_rss_mb"]),
            ));
        }
    }
    let p = &d["project"];
    s.push_str(&format!(
        "\n### Whole project\n\nAll {} ABAP files in one run. origami: the CLI with `--no-config --check` and the file paths; cold is the first run, warm the median of the {} runs after it. ABAP Cleaner: one JVM over the same files, one thread; cold is the first pass including the JVM start (wall clock), warm the third pass. The operating system's file cache is warm for both, since the inputs were read before.\n\n| Tool | cold | warm | warm vs origami `-j 1` | peak memory (MB) |\n|---|---:|---:|---:|---:|\n",
        i["project"]["count"], PROJECT_RUNS
    ));
    let o1 = &p["origami_j1"]["warm"]["p50"];
    s.push_str(&format!(
        "| origami, 1 thread (`-j 1`) | {} | {} | – | {} |\n",
        secs(&p["origami_j1"]["cold_ms"]),
        secs(o1),
        mb(&p["origami_j1"]["peak_rss_mb"])
    ));
    s.push_str(&format!(
        "| origami, one thread per core ({}) | {} | {} | – | {} |\n",
        m["cores"].as_str().unwrap_or(""),
        secs(&p["origami"]["cold_ms"]),
        secs(&p["origami"]["warm"]["p50"]),
        mb(&p["origami"]["peak_rss_mb"])
    ));
    for (t, name) in &TOOLS[1..] {
        let x = &p[*t];
        s.push_str(&format!(
            "| {name} (`-Xmx{}`) | {} | {} | {} | {} |\n",
            x["xmx"].as_str().unwrap_or(""),
            secs(&x["cold_ms"]),
            secs(&x["warm_ms"]),
            times(&x["warm_ms"], o1),
            mb(&x["peak_rss_mb"])
        ));
    }
    s.push_str("\n### How to read this\n\n");
    s.push_str("* **Cold** is what a single command costs: starting the program, reading the input, formatting it once. A JVM needs a few hundred milliseconds to start and load ABAP Cleaner before it formats anything, so for one statement or one method the cold time is almost all start-up. origami is a native program and starts in about a millisecond.\n");
    s.push_str("* **Warm** is what a call costs once the program is running and, for Java, the JIT compiler has compiled the hot code, as in an editor plug-in or a server. This compares the formatting work itself.\n");
    s.push_str("* **Whole project** is one command over many files, as in CI or a pre-commit run over a repository. Here the start-up is paid once; ABAP Cleaner's harness runs on one thread, so the fairest comparison is origami with `-j 1`.\n");
    s.push_str("* **The tools do different work.** The default profile of ABAP Cleaner applies about 100 rules, several of which change code, so it does more than origami. The layout profile (the default profile reduced to its layout rules, `Compare.layoutProfile()` in `scripts/abap-cleaner-compare/Compare.java`) only changes whitespace, line breaks and letter case, as origami does; it is the closer comparison. Neither profile produces the same layout as origami.\n");
    s.push_str("* ABAP Cleaner is run through its Java API, without the Eclipse start of its launcher, so its cold times are a lower bound.\n\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn methods_are_cut_as_written_without_amdp() {
        let code = "CLASS c IMPLEMENTATION.\n  METHOD a.\n    x = 1.\n  ENDMETHOD.\n\
                    * METHOD commented.\n  METHOD b BY DATABASE PROCEDURE FOR HDB LANGUAGE SQLSCRIPT.\n    select 1 from dummy;\n  ENDMETHOD.\nENDCLASS.\n";
        assert_eq!(methods(code), ["  METHOD a.\n    x = 1.\n  ENDMETHOD.\n"]);
    }

    #[test]
    fn statements_are_complete_and_alone() {
        let m = concat!(
            "  METHOD a.\n",
            "* comment\n",
            "    DATA: lv_a TYPE i,\n",
            "          lv_b TYPE string. \" end\n",
            "    lv_a = 1. lv_b = `x.y`.\n",
            "    IF lv_a = 1.\n",
            "      lv_b = |a.{ lv_a }\\|.| && 'b.c'.\n",
            "    ENDIF.\n",
            "    DATA BEGIN OF ls.\n",
            "    CALL METHOD me->x\n",
            "      \" inside\n",
            "      EXPORTING a = 1.\n",
            "  ENDMETHOD.\n",
        );
        assert_eq!(
            statements(m),
            [
                "    DATA: lv_a TYPE i,\n          lv_b TYPE string. \" end\n",
                "      lv_b = |a.{ lv_a }\\|.| && 'b.c'.\n",
                "    CALL METHOD me->x\n      \" inside\n      EXPORTING a = 1.\n",
            ]
        );
    }

    #[test]
    fn every_kth_keeps_about_n() {
        assert_eq!(every_kth((0..10).collect(), 5), [0, 2, 4, 6, 8]);
        assert_eq!(every_kth((0..3).collect(), 5), [0, 1, 2]);
    }
}

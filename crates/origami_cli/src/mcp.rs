//! `origami --mcp`: a Model Context Protocol server on stdin/stdout
//! (JSON-RPC 2.0, one message per line). Tools:
//!
//! * `format`: source text in, formatted text and diagnostics out;
//! * `format_files`: check (default) or rewrite files and directories,
//!   optionally only the lines changed against a git revision.
//!
//! Formatting, configuration discovery and the self-check are the same as
//! on the command line. Nothing is written unless `format_files` gets
//! `"write": true`.
//!
//! Tool calls run one at a time on a worker thread, each with a deadline
//! (30 s, `ORIGAMI_MCP_TIMEOUT` seconds); other requests are answered while
//! a call runs, and `notifications/cancelled` cancels a call. A message may
//! be at most 16 MiB long.

use crate::args::Args;
use crate::report::diff::unified;
use crate::run::{Configs, format_source};
use origami_core::detect::LanguageHint;
use origami_core::{Language, Status};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

const PROTOCOLS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// The longest message (one line) the server accepts, in bytes.
const MAX_MESSAGE: usize = 16 * 1024 * 1024;

/// The deadline for one tool call unless `ORIGAMI_MCP_TIMEOUT` sets one.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// About how many bytes of file entries and diffs one `format_files` result
/// carries; the rest is summarised.
const RESPONSE_BUDGET: usize = 100 * 1024;

/// JSON-RPC error code for a tool call that missed its deadline.
const TIMED_OUT: i64 = -32001;

enum Event {
    /// One message from stdin, without the line break.
    Line(Vec<u8>),
    /// A message longer than `MAX_MESSAGE`; the start of it, to find the id.
    TooLong(Vec<u8>),
    /// stdin ended (exit code 0) or could not be read (exit code 4).
    End(i32),
    /// A tool call finished.
    Done { seq: u64, reply: Value },
}

/// The tool call in progress.
struct Running {
    seq: u64,
    id: Value,
    cancel: Arc<AtomicBool>,
    deadline: Instant,
    writes: bool,
}

pub fn serve() -> i32 {
    let timeout = timeout();
    let (tx, rx) = mpsc::channel();
    let reader = tx.clone();
    if std::thread::Builder::new()
        .name("origami-mcp-stdin".into())
        .spawn(move || read_messages(std::io::stdin().lock(), &reader))
        .is_err()
    {
        eprintln!("origami: cannot start the MCP server (no thread for stdin)");
        return 70;
    }
    let mut stdout = std::io::stdout().lock();
    let mut send = |reply: &Value| {
        writeln!(stdout, "{reply}")
            .and_then(|_| stdout.flush())
            .is_ok()
    };
    let mut running: Option<Running> = None;
    let mut queue: VecDeque<Value> = VecDeque::new();
    let mut seq = 0;
    let mut end = None;
    loop {
        if running.is_none() {
            if let Some(msg) = queue.pop_front() {
                seq += 1;
                match start(msg, seq, timeout, &tx) {
                    Ok(r) => running = Some(r),
                    Err(reply) if !send(&reply) => return 4,
                    Err(_) => {}
                }
                continue;
            }
            if let Some(code) = end {
                return code;
            }
        }
        let event = match &running {
            Some(r) => {
                let left = r.deadline.saturating_duration_since(Instant::now());
                match (!left.is_zero()).then(|| rx.recv_timeout(left)) {
                    Some(Ok(e)) => e,
                    Some(Err(RecvTimeoutError::Disconnected)) => return 70,
                    None | Some(Err(RecvTimeoutError::Timeout)) => {
                        if let Some(r) = running.take() {
                            r.cancel.store(true, Ordering::Relaxed);
                            if !send(&timed_out(&r, timeout)) {
                                return 4;
                            }
                        }
                        continue;
                    }
                }
            }
            None => match rx.recv() {
                Ok(e) => e,
                Err(_) => return 70,
            },
        };
        let reply = match event {
            Event::End(code) => {
                end = Some(code);
                None
            }
            Event::Done { seq, reply } => match &running {
                Some(r) if r.seq == seq => {
                    running = None;
                    Some(reply)
                }
                // A call that timed out or was cancelled: no reply.
                _ => None,
            },
            Event::TooLong(start) => Some(error(
                id_in(&start),
                -32600,
                &format!(
                    "Message too long: more than {} MiB on one line; nothing was done. Send smaller sources, or use format_files with paths for large files.",
                    MAX_MESSAGE >> 20
                ),
            )),
            Event::Line(line) if line.trim_ascii().is_empty() => None,
            Event::Line(line) => match serde_json::from_slice::<Value>(&line) {
                Err(e) => Some(error(Value::Null, -32700, &format!("Parse error: {e}"))),
                Ok(msg) if msg["method"] == "notifications/cancelled" => {
                    let id = &msg["params"]["requestId"];
                    if running.as_ref().is_some_and(|r| &r.id == id)
                        && let Some(r) = running.take()
                    {
                        r.cancel.store(true, Ordering::Relaxed);
                    }
                    queue.retain(|m| &m["id"] != id);
                    None
                }
                Ok(msg) if msg["method"] == "tools/call" && msg.get("id").is_some() => {
                    queue.push_back(msg);
                    None
                }
                Ok(msg) => handle(&msg, &AtomicBool::new(false)),
            },
        };
        if let Some(r) = reply
            && !send(&r)
        {
            return 4;
        }
    }
}

/// The deadline for one tool call: `ORIGAMI_MCP_TIMEOUT` (seconds) or 30 s.
fn timeout() -> Duration {
    let Ok(v) = std::env::var("ORIGAMI_MCP_TIMEOUT") else {
        return DEFAULT_TIMEOUT;
    };
    match v
        .trim()
        .parse::<f64>()
        .ok()
        .and_then(|s| Duration::try_from_secs_f64(s).ok())
    {
        Some(d) => d,
        None => {
            eprintln!("origami: ignoring ORIGAMI_MCP_TIMEOUT={v} (not a number of seconds)");
            DEFAULT_TIMEOUT
        }
    }
}

/// Starts a tool call on a worker thread; `Err` is the reply if it cannot.
fn start(
    msg: Value,
    seq: u64,
    timeout: Duration,
    tx: &mpsc::Sender<Event>,
) -> Result<Running, Value> {
    let id = msg["id"].clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let running = Running {
        seq,
        id: id.clone(),
        cancel: Arc::clone(&cancel),
        deadline: Instant::now() + timeout,
        writes: msg["params"]["arguments"]["write"] == true,
    };
    let tx = tx.clone();
    let worker = move || {
        let reply = crate::panic::catch(|| handle(&msg, &cancel))
            .map(|r| r.unwrap_or(Value::Null))
            .unwrap_or_else(|m| error(id, -32603, &format!("Internal error (X002): {m}")));
        let _ = tx.send(Event::Done { seq, reply });
    };
    match std::thread::Builder::new()
        .name("origami-mcp-call".into())
        .spawn(worker)
    {
        Ok(_) => Ok(running),
        Err(e) => Err(error(
            running.id,
            -32603,
            &format!("Internal error: cannot start a worker thread: {e}"),
        )),
    }
}

fn timed_out(r: &Running, timeout: Duration) -> Value {
    let mut message = format!(
        "Request timed out after {} s and was cancelled. Call again with less work: fewer paths or a smaller source.",
        timeout.as_secs_f64()
    );
    if r.writes {
        message.push_str(" Files rewritten before the deadline stay rewritten; call format_files without write to see what is left.");
    }
    json!({"jsonrpc": "2.0", "id": r.id, "error": {"code": TIMED_OUT, "message": message}})
}

/// Sends each line of `input` to `tx` until the end of the input.
fn read_messages(mut input: impl BufRead, tx: &mpsc::Sender<Event>) {
    loop {
        let event = match read_line(&mut input, MAX_MESSAGE) {
            Ok(Some(Ok(line))) => Event::Line(line),
            Ok(Some(Err(start))) => Event::TooLong(start),
            Ok(None) => Event::End(0),
            Err(_) => Event::End(4),
        };
        let last = matches!(event, Event::End(_));
        if tx.send(event).is_err() || last {
            return;
        }
    }
}

/// Reads one line without its `\n`. `None` at the end of the input;
/// `Err` with the first bytes for a line longer than `max`, which is read
/// to its end but not kept.
fn read_line(
    input: &mut impl BufRead,
    max: usize,
) -> std::io::Result<Option<Result<Vec<u8>, Vec<u8>>>> {
    const KEEP: usize = 4096;
    let mut line = Vec::new();
    let mut too_long = false;
    let mut any = false;
    loop {
        let chunk = match input.fill_buf() {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if chunk.is_empty() {
            return Ok(any.then_some(if too_long { Err(line) } else { Ok(line) }));
        }
        any = true;
        let newline = chunk.iter().position(|&b| b == b'\n');
        let part = &chunk[..newline.unwrap_or(chunk.len())];
        if !too_long && line.len() + part.len() > max {
            too_long = true;
            line.truncate(KEEP);
        }
        if too_long {
            let room = KEEP.saturating_sub(line.len()).min(part.len());
            line.extend_from_slice(&part[..room]);
        } else {
            line.extend_from_slice(part);
        }
        let used = newline.map_or(chunk.len(), |i| i + 1);
        input.consume(used);
        if newline.is_some() {
            return Ok(Some(if too_long { Err(line) } else { Ok(line) }));
        }
    }
}

/// The `id` of a message from its first bytes (before `params`), if any.
fn id_in(start: &[u8]) -> Value {
    let text = String::from_utf8_lossy(start);
    let head = text.split("\"params\"").next().unwrap_or("");
    let Some(i) = head.find("\"id\"") else {
        return Value::Null;
    };
    let rest = head[i + 4..].trim_start();
    let Some(rest) = rest.strip_prefix(':') else {
        return Value::Null;
    };
    let rest = rest.trim_start();
    let end = if let Some(s) = rest.strip_prefix('"') {
        s.find('"').map(|j| j + 2)
    } else {
        rest.find(|c: char| !(c.is_ascii_digit() || c == '-'))
    };
    end.and_then(|e| serde_json::from_str(&rest[..e]).ok())
        .unwrap_or(Value::Null)
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn ok(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Answers one message; notifications get no answer. A tool call stops
/// early once `cancel` is set (its answer is then dropped).
fn handle(msg: &Value, cancel: &AtomicBool) -> Option<Value> {
    let method = msg.get("method")?.as_str()?;
    let id = msg.get("id").cloned()?;
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    Some(match method {
        "initialize" => {
            let asked = params["protocolVersion"].as_str().unwrap_or("");
            let version = if PROTOCOLS.contains(&asked) {
                asked
            } else {
                PROTOCOLS[0]
            };
            ok(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "origami", "version": env!("CARGO_PKG_VERSION")},
                    "instructions": "Layout-only formatter for ABAP and the CDS family. Use `format` for source text and `format_files` for files in the workspace. Formatting never changes tokens: it re-lexes its own output and refuses to write if anything but layout changed."
                }),
            )
        }
        "ping" => ok(id, json!({})),
        "tools/list" => ok(id, json!({"tools": tools()})),
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or("");
            let a = &params["arguments"];
            let result = match name {
                "format" => format_tool(a),
                "format_files" => files_tool(a, cancel),
                _ => Err(format!("Unknown tool: {name}")),
            };
            match result {
                Ok(r) => ok(id, r),
                Err(m) => error(id, -32602, &m),
            }
        }
        _ => error(id, -32601, &format!("Method not found: {method}")),
    })
}

fn tools() -> Value {
    json!([
        {
            "name": "format",
            "title": "Format ABAP or CDS source",
            "description": "Formats ABAP or CDS-family source (DDLS, DDLX, DCL, BDEF, SRVD) and returns the formatted text. Layout only: tokens, identifiers and comments never change. On a structural error (e.g. a missing period) nothing is returned and the diagnostics say where.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": {"type": "string", "description": "The source text."},
                    "filename": {"type": "string", "description": "abapGit-style name used to detect the language and find origami.toml, e.g. zcl_foo.clas.abap or zi_travel.ddls.asddls."},
                    "language": {"type": "string", "enum": ["abap", "ddls", "ddlx", "dcl", "bdef", "srvd", "cds"], "description": "Overrides detection from the filename."},
                    "lines": {"type": "array", "items": {"type": "array", "items": {"type": "integer", "minimum": 1}, "minItems": 2, "maxItems": 2}, "description": "Only format changes touching these 1-based inclusive line ranges, e.g. [[10, 25]]."}
                },
                "required": ["source"]
            },
            "annotations": {"readOnlyHint": true, "idempotentHint": true, "openWorldHint": false}
        },
        {
            "name": "format_files",
            "title": "Check or format files",
            "description": "Checks (default) or rewrites ABAP and CDS files and directories below the server's working directory. Without write, returns which files would change and a unified diff for each. With changed, only lines changed against that git revision (and untracked files) are formatted. A large result is shortened (truncated: true); call again with fewer paths for the rest.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paths": {"type": "array", "items": {"type": "string"}, "description": "Files or directories; default: the working directory."},
                    "write": {"type": "boolean", "description": "Rewrite changed files in place. Default false."},
                    "changed": {"type": "string", "description": "Git revision, e.g. HEAD or main: only format lines changed against it."}
                }
            },
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "idempotentHint": true, "openWorldHint": false}
        }
    ])
}

fn diagnostics_json(diags: &[origami_core::Reported]) -> Value {
    serde_json::to_value(diags).unwrap_or(Value::Null)
}

fn summary(diags: &[origami_core::Reported]) -> String {
    diags
        .iter()
        .map(|d| {
            format!(
                "{}:{} {} {}",
                d.range.start.line,
                d.range.start.column,
                d.code.as_str(),
                d.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn tool_error(text: String, structured: Value) -> Value {
    json!({"content": [{"type": "text", "text": text}], "structuredContent": structured, "isError": true})
}

/// The optional argument `key` of the tool call `a`, as a value of the type
/// the tool schema declares. A wrong type is an error: silently using the
/// default would, for `paths` with `write`, rewrite the whole working directory.
fn optional<'a, T>(
    a: &'a Value,
    key: &str,
    expected: &str,
    get: impl Fn(&'a Value) -> Option<T>,
) -> Result<Option<T>, String> {
    match &a[key] {
        Value::Null => Ok(None),
        v => get(v)
            .map(Some)
            .ok_or_else(|| format!("`{key}` must be {expected}; got {v}")),
    }
}

/// The tool arguments must be an object (or absent).
fn check_object(a: &Value) -> Result<(), String> {
    if a.is_null() || a.is_object() {
        Ok(())
    } else {
        Err(format!("`arguments` must be an object; got {a}"))
    }
}

/// The `lines` argument: `[[start, end], …]`, 1 <= start <= end < 2^32.
fn line_ranges(v: &Value) -> Result<Vec<(u32, u32)>, String> {
    if v.is_null() {
        return Ok(Vec::new());
    }
    let bad = || {
        format!(
            "`lines` must be [[start, end], …] with 1 <= start <= end <= {}; got {v}",
            u32::MAX
        )
    };
    let ranges = v.as_array().ok_or_else(bad)?;
    ranges
        .iter()
        .map(|r| {
            let line = |i: usize| {
                r.as_array()
                    .filter(|a| a.len() == 2)
                    .and_then(|a| a[i].as_u64())
                    .and_then(|n| u32::try_from(n).ok())
            };
            match (line(0), line(1)) {
                (Some(a), Some(b)) if a >= 1 && a <= b => Ok((a, b)),
                _ => Err(bad()),
            }
        })
        .collect()
}

fn format_tool(a: &Value) -> Result<Value, String> {
    check_object(a)?;
    let source = optional(a, "source", "a string", Value::as_str)?;
    let filename = optional(a, "filename", "a string", Value::as_str)?;
    let language = optional(a, "language", "a string", Value::as_str)?;
    let lines = line_ranges(&a["lines"])?;
    let Some(source) = source else {
        return Ok(tool_error("`source` is required".into(), json!({})));
    };
    let hint = match language {
        Some("cds") => LanguageHint::CdsFamily,
        Some(l) => match Language::parse(l) {
            Some(lang) => LanguageHint::Explicit(lang),
            None => return Ok(tool_error(format!("Unknown language `{l}`"), json!({}))),
        },
        None => match filename {
            Some(n) => LanguageHint::StdinPath(n.to_string()),
            None => LanguageHint::None,
        },
    };
    let dir = filename
        .and_then(|n| Path::new(n).parent().map(Path::to_path_buf))
        .filter(|d| !d.as_os_str().is_empty() && d.is_dir())
        .unwrap_or_else(|| PathBuf::from("."));
    let args = Args::default();
    let opts = match Configs::new(&args).for_dir(&dir) {
        Ok((o, _)) => o,
        Err(issue) => return Ok(tool_error(issue.message, json!({}))),
    };
    let r = format_source(source.as_bytes(), &hint, &opts, &lines);
    let diags = diagnostics_json(&r.diagnostics);
    Ok(match (&r.status, r.output) {
        (Status::Formatted | Status::Unchanged, Some(out)) => {
            let text = String::from_utf8_lossy(&out).into_owned();
            json!({
                "content": [{"type": "text", "text": text.clone()}],
                "structuredContent": {"source": text, "changed": r.changed, "status": r.status, "diagnostics": diags},
                "isError": false
            })
        }
        _ => tool_error(
            format!(
                "origami could not format the source (left unchanged):\n{}",
                summary(&r.diagnostics)
            ),
            json!({"changed": false, "status": r.status, "diagnostics": diags}),
        ),
    })
}

/// The paths (as given) that do not resolve to a place below the current
/// working directory. A path that does not exist is resolved through its
/// nearest existing parent.
fn outside_cwd<'a>(paths: impl Iterator<Item = &'a PathBuf>) -> Vec<String> {
    let Ok(cwd) = std::env::current_dir().and_then(|d| d.canonicalize()) else {
        return paths.map(|p| p.display().to_string()).collect();
    };
    paths
        .filter(|p| {
            let abs = if p.is_absolute() {
                p.to_path_buf()
            } else {
                cwd.join(p)
            };
            // Resolve the longest existing prefix; `..` in the rest counts.
            let mut base = abs.as_path();
            let mut rest = Vec::new();
            let resolved = loop {
                if let Ok(c) = base.canonicalize() {
                    break Some(c);
                }
                match (base.parent(), base.file_name()) {
                    (Some(parent), Some(name)) => {
                        rest.push(name.to_os_string());
                        base = parent;
                    }
                    _ => break None,
                }
            };
            let Some(mut full) = resolved else {
                return true;
            };
            if rest.iter().any(|r| r == "..") {
                return true;
            }
            for part in rest.iter().rev() {
                full.push(part);
            }
            !full.starts_with(&cwd)
        })
        .map(|p| p.display().to_string())
        .collect()
}

fn files_tool(a: &Value, cancel: &AtomicBool) -> Result<Value, String> {
    check_object(a)?;
    let paths: Vec<PathBuf> = optional(a, "paths", "an array of strings", |v| {
        v.as_array()?
            .iter()
            .map(|p| p.as_str().map(PathBuf::from))
            .collect()
    })?
    .unwrap_or_default();
    let write = optional(a, "write", "a boolean", Value::as_bool)?.unwrap_or(false);
    let changed = optional(a, "changed", "a string", Value::as_str)?;
    Ok(files_run(&paths, write, changed, cancel))
}

fn files_run(paths: &[PathBuf], write: bool, changed: Option<&str>, cancel: &AtomicBool) -> Value {
    // The tool works only below the server's working directory.
    let outside = outside_cwd(paths.iter());
    if !outside.is_empty() {
        return tool_error(
            format!(
                "Paths must lie below the server's working directory; nothing was done: {}",
                outside.join(", ")
            ),
            json!({"outside": outside}),
        );
    }
    let selected: Vec<(PathBuf, crate::changed::Ranges)> = match changed {
        Some(rev) => match crate::changed::files(rev, paths) {
            Ok(f) => f,
            Err(m) => return tool_error(m, json!({})),
        },
        None => {
            let roots = if paths.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                paths.to_vec()
            };
            crate::walk::files(&roots)
                .into_iter()
                .map(|f| (f, None))
                .collect()
        }
    };
    // Files reached through symbolic links must stay inside too.
    let outside = outside_cwd(selected.iter().map(|(p, _)| p));
    if !outside.is_empty() {
        return tool_error(
            format!(
                "Files outside the server's working directory (symbolic links?); nothing was done: {}",
                outside.join(", ")
            ),
            json!({"outside": outside}),
        );
    }
    let cancelled = || tool_error("Cancelled".into(), json!({}));
    let args = Args::default();
    let configs = Configs::new(&args);
    // Per file: the entry, its diff, and the text for errors.
    let mut results: Vec<(Value, Option<String>, String)> = Vec::new();
    let (mut n_changed, mut n_error) = (0, 0);
    for (path, ranges) in &selected {
        if cancel.load(Ordering::Relaxed) {
            return cancelled();
        }
        let display = path.strip_prefix(".").unwrap_or(path).display().to_string();
        let dir = path
            .parent()
            .filter(|d| !d.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let opts = match configs.for_dir(&dir) {
            Ok((o, _)) => o,
            Err(issue) => return tool_error(issue.message, json!({})),
        };
        let input = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                n_error += 1;
                let entry = json!({"path": display, "status": "error", "changed": false, "message": format!("Cannot read file: {e}")});
                results.push((entry, None, String::new()));
                continue;
            }
        };
        let r = format_source(
            &input,
            &LanguageHint::Path(display.clone()),
            &opts,
            ranges.as_deref().unwrap_or(&[]),
        );
        let mut entry = json!({"path": display, "status": r.status, "changed": r.changed, "diagnostics": diagnostics_json(&r.diagnostics)});
        let mut note = String::new();
        let mut diff = None;
        if r.status == Status::Error {
            n_error += 1;
            note = format!("{display}: not formatted\n{}\n", summary(&r.diagnostics));
        }
        if let (true, Some(out)) = (r.changed, &r.output) {
            n_changed += 1;
            if write {
                if cancel.load(Ordering::Relaxed) {
                    return cancelled();
                }
                if let Err(e) = crate::io::write_atomic(path, out) {
                    n_error += 1;
                    entry["status"] = json!("error");
                    entry["message"] = json!(format!("Cannot write file: {e}"));
                }
            } else {
                diff = Some(unified(&display, &input, out));
            }
        }
        results.push((entry, diff, note));
    }
    let total = results.len();
    let (files, mut text, truncated) = within_budget(results);
    let head = format!(
        "{} file(s) processed, {} {}, {} with errors",
        total,
        n_changed,
        if write { "rewritten" } else { "would change" },
        n_error
    );
    text.insert_str(0, &format!("{head}\n"));
    let mut structured =
        json!({"files": files, "changed": n_changed, "errors": n_error, "written": write});
    if let Some(t) = truncated {
        text.push_str(&t.note());
        structured["truncated"] = json!(true);
        structured["omitted"] = json!(t.omitted);
    }
    json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": structured,
        "isError": false
    })
}

/// What `within_budget` left out.
struct Truncated {
    /// Files listed without their diff.
    no_diff: Vec<String>,
    /// Files not listed at all.
    omitted: usize,
}

impl Truncated {
    fn note(&self) -> String {
        const SHOWN: usize = 20;
        let mut s = format!("\nResult shortened to about {} KB:", RESPONSE_BUDGET / 1024);
        if !self.no_diff.is_empty() {
            let names = self.no_diff.iter().take(SHOWN).cloned().collect::<Vec<_>>();
            s.push_str(&format!(
                " no diff for {} changed file(s) (`diff_omitted`): {}",
                self.no_diff.len(),
                names.join(", ")
            ));
            if self.no_diff.len() > SHOWN {
                s.push_str(&format!(" and {} more", self.no_diff.len() - SHOWN));
            }
            s.push(';');
        }
        if self.omitted > 0 {
            s.push_str(&format!(" {} more file(s) not listed;", self.omitted));
        }
        s.push_str(" the counts above cover all files. To see the rest, call format_files again with `paths` set to fewer files or directories.\n");
        s
    }
}

/// The file entries and the text with the diffs and error notes, in order,
/// as far as they fit into `RESPONSE_BUDGET`. A diff counts twice (it is in
/// the text and in the entry). After the first diff that does not fit, the
/// entries carry `diff_omitted`; after the first entry that does not fit,
/// the rest is only counted.
fn within_budget(
    results: Vec<(Value, Option<String>, String)>,
) -> (Vec<Value>, String, Option<Truncated>) {
    let mut used = 0;
    let mut files = Vec::new();
    let mut text = String::new();
    let mut t = Truncated {
        no_diff: Vec::new(),
        omitted: 0,
    };
    for (mut entry, diff, note) in results {
        let size = entry.to_string().len() + note.len();
        if t.omitted > 0 || used + size > RESPONSE_BUDGET {
            t.omitted += 1;
            continue;
        }
        used += size;
        text.push_str(&note);
        if let Some(d) = diff {
            if t.no_diff.is_empty() && used + 2 * d.len() <= RESPONSE_BUDGET {
                used += 2 * d.len();
                text.push_str(&d);
                entry["diff"] = json!(d);
            } else {
                entry["diff_omitted"] = json!(true);
                t.no_diff
                    .push(entry["path"].as_str().unwrap_or_default().to_string());
            }
        }
        files.push(entry);
    }
    let truncated = (t.omitted > 0 || !t.no_diff.is_empty()).then_some(t);
    (files, text, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_files_is_announced_as_destructive() {
        let tools = tools();
        let annotations = |name: &str| {
            tools
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t["name"] == name)
                .map(|t| t["annotations"].clone())
                .unwrap()
        };
        // It rewrites files with `write: true`.
        let files = annotations("format_files");
        assert_eq!(files["readOnlyHint"], false);
        assert_eq!(files["destructiveHint"], true);
        assert_eq!(annotations("format")["readOnlyHint"], true);
    }

    #[test]
    fn read_line_caps_long_lines_and_keeps_reading() {
        let mut input: &[u8] = b"{\"id\":7,\"x\":\"aaaaaaaaaaaa\"}\nok\nlast";
        assert!(
            matches!(read_line(&mut input, 10), Ok(Some(Err(start))) if start.starts_with(b"{\"id\":7"))
        );
        assert_eq!(read_line(&mut input, 10).unwrap(), Some(Ok(b"ok".to_vec())));
        assert_eq!(
            read_line(&mut input, 10).unwrap(),
            Some(Ok(b"last".to_vec()))
        );
        assert_eq!(read_line(&mut input, 10).unwrap(), None);
    }

    #[test]
    fn id_in_finds_the_request_id() {
        assert_eq!(id_in(br#"{"jsonrpc":"2.0","id":42,"method":"#), json!(42));
        assert_eq!(id_in(br#"{"id" : "a-1","params":{"#), json!("a-1"));
        assert_eq!(id_in(br#"{"params":{"id":3"#), Value::Null);
    }

    #[test]
    fn line_ranges_are_validated() {
        assert_eq!(
            line_ranges(&json!([[1, 2], [5, 5]])),
            Ok(vec![(1, 2), (5, 5)])
        );
        assert_eq!(line_ranges(&Value::Null), Ok(vec![]));
        for bad in [
            json!([[0, 1]]),
            json!([[3, 2]]),
            json!([[1, 4_294_967_296_u64]]),
            json!([[1]]),
            json!([[1, 2, 3]]),
            json!([["1", "2"]]),
            json!([[-1, 2]]),
            json!("1:2"),
        ] {
            assert!(line_ranges(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn within_budget_drops_diffs_then_entries() {
        let big = "x".repeat(RESPONSE_BUDGET / 3);
        let entry = |p: &str| json!({"path": p});
        let results = vec![
            (entry("a"), Some(big.clone()), String::new()),
            (entry("b"), Some(big.clone()), String::new()),
            (entry("c"), None, String::new()),
        ];
        let (files, text, t) = within_budget(results);
        assert_eq!(files.len(), 3);
        assert!(files[0].get("diff").is_some());
        assert_eq!(files[1]["diff_omitted"], true);
        assert_eq!(text.len(), big.len());
        let t = t.unwrap();
        assert_eq!((t.no_diff.clone(), t.omitted), (vec!["b".to_string()], 0));
        let many = (0..10_000)
            .map(|i| (entry(&format!("f{i}")), None, String::new()))
            .collect();
        let (files, _, t) = within_budget(many);
        assert!(files.len() < 10_000);
        assert_eq!(t.unwrap().omitted, 10_000 - files.len());
    }
}

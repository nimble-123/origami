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

use crate::args::Args;
use crate::report::diff::unified;
use crate::run::Configs;
use clap::Parser;
use origami_core::detect::LanguageHint;
use origami_core::{Language, Status, format_lines};
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

const PROTOCOLS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

pub fn serve() -> i32 {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { return 4 };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(&msg),
            Err(e) => Some(error(Value::Null, -32700, &format!("Parse error: {e}"))),
        };
        if let Some(r) = reply
            && writeln!(stdout, "{r}")
                .and_then(|_| stdout.flush())
                .is_err()
        {
            return 4;
        }
    }
    0
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn ok(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Answers one message; notifications get no answer.
pub fn handle(msg: &Value) -> Option<Value> {
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
            match name {
                "format" => ok(id, format_tool(a)),
                "format_files" => ok(id, files_tool(a)),
                _ => error(id, -32602, &format!("Unknown tool: {name}")),
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
            "description": "Checks (default) or rewrites ABAP and CDS files and directories below the server's working directory. Without write, returns which files would change and a unified diff for each. With changed, only lines changed against that git revision (and untracked files) are formatted.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paths": {"type": "array", "items": {"type": "string"}, "description": "Files or directories; default: the working directory."},
                    "write": {"type": "boolean", "description": "Rewrite changed files in place. Default false."},
                    "changed": {"type": "string", "description": "Git revision, e.g. HEAD or main: only format lines changed against it."}
                }
            },
            "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
        }
    ])
}

fn default_args() -> Args {
    Args::try_parse_from(["origami"]).expect("default arguments")
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

fn format_tool(a: &Value) -> Value {
    let Some(source) = a["source"].as_str() else {
        return tool_error("`source` is required".into(), json!({}));
    };
    let filename = a["filename"].as_str();
    let hint = match a["language"].as_str() {
        Some("cds") => LanguageHint::CdsFamily,
        Some(l) => match Language::parse(l) {
            Some(lang) => LanguageHint::Explicit(lang),
            None => return tool_error(format!("Unknown language `{l}`"), json!({})),
        },
        None => match filename {
            Some(n) => LanguageHint::StdinPath(n.to_string()),
            None => LanguageHint::None,
        },
    };
    let lines: Vec<(u32, u32)> = a["lines"]
        .as_array()
        .map(|v| {
            v.iter()
                .filter_map(|r| Some((r[0].as_u64()? as u32, r[1].as_u64()? as u32)))
                .collect()
        })
        .unwrap_or_default();
    let dir = filename
        .and_then(|n| Path::new(n).parent().map(Path::to_path_buf))
        .filter(|d| !d.as_os_str().is_empty() && d.is_dir())
        .unwrap_or_else(|| PathBuf::from("."));
    let args = default_args();
    let opts = match Configs::new(&args).for_dir(&dir) {
        Ok((o, _)) => o,
        Err(issue) => return tool_error(issue.message, json!({})),
    };
    let r = format_lines(source.as_bytes(), &hint, &opts, &lines);
    let diags = diagnostics_json(&r.diagnostics);
    match (&r.status, r.output) {
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
    }
}

fn files_tool(a: &Value) -> Value {
    let paths: Vec<PathBuf> = a["paths"]
        .as_array()
        .map(|v| {
            v.iter()
                .filter_map(|p| p.as_str().map(PathBuf::from))
                .collect()
        })
        .unwrap_or_default();
    let write = a["write"].as_bool().unwrap_or(false);
    let selected: Vec<(PathBuf, crate::changed::Ranges)> = match a["changed"].as_str() {
        Some(rev) => match crate::changed::files(rev, &paths) {
            Ok(f) => f,
            Err(m) => return tool_error(m, json!({})),
        },
        None => {
            let roots = if paths.is_empty() {
                vec![PathBuf::from(".")]
            } else {
                paths
            };
            crate::walk::files(&roots)
                .into_iter()
                .map(|f| (f, None))
                .collect()
        }
    };
    let args = default_args();
    let configs = Configs::new(&args);
    let mut files = Vec::new();
    let mut text = String::new();
    let (mut n_changed, mut n_error) = (0, 0);
    for (path, ranges) in &selected {
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
                files.push(json!({"path": display, "status": "error", "changed": false, "message": format!("Cannot read file: {e}")}));
                continue;
            }
        };
        let r = format_lines(
            &input,
            &LanguageHint::Path(display.clone()),
            &opts,
            ranges.as_deref().unwrap_or(&[]),
        );
        let mut entry = json!({"path": display, "status": r.status, "changed": r.changed, "diagnostics": diagnostics_json(&r.diagnostics)});
        if r.status == Status::Error {
            n_error += 1;
            text.push_str(&format!(
                "{display}: not formatted\n{}\n",
                summary(&r.diagnostics)
            ));
        }
        if let (true, Some(out)) = (r.changed, &r.output) {
            n_changed += 1;
            if write {
                if let Err(e) = crate::io::write_atomic(path, out) {
                    n_error += 1;
                    entry["status"] = json!("error");
                    entry["message"] = json!(format!("Cannot write file: {e}"));
                }
            } else {
                let d = unified(&display, &input, out);
                text.push_str(&d);
                entry["diff"] = json!(d);
            }
        }
        files.push(entry);
    }
    let head = format!(
        "{} file(s) processed, {} {}, {} with errors",
        files.len(),
        n_changed,
        if write { "rewritten" } else { "would change" },
        n_error
    );
    json!({
        "content": [{"type": "text", "text": format!("{head}\n{text}")}],
        "structuredContent": {"files": files, "changed": n_changed, "errors": n_error, "written": write},
        "isError": false
    })
}

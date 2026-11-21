//! SARIF 2.1.0 output, one rule per diagnostic code.

use super::Run;
use origami_core::{Code, Severity};
use serde_json::{Value, json};
use std::io::Write;

pub fn write(run: &Run, out: &mut dyn Write) -> std::io::Result<()> {
    let (files, _) = run.capped();
    let rules: Vec<Value> = Code::all()
        .iter()
        .map(|c| json!({ "id": c.as_str() }))
        .collect();
    let mut results = Vec::new();
    for (f, diags) in files {
        for d in diags {
            let level = match d.severity {
                Severity::Info => "note",
                Severity::Warning => "warning",
                Severity::Error | Severity::Internal => "error",
            };
            results.push(json!({
                "ruleId": d.code.as_str(),
                "level": level,
                "message": { "text": d.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": f.path },
                        "region": {
                            "startLine": d.range.start.line,
                            "startColumn": d.range.start.column,
                            "endLine": d.range.end.line,
                            "endColumn": d.range.end.column
                        }
                    }
                }]
            }));
        }
    }
    let doc = json!({
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [{
            "tool": { "driver": {
                "name": "origami",
                "version": env!("CARGO_PKG_VERSION"),
                "rules": rules
            }},
            "results": results
        }]
    });
    serde_json::to_writer(&mut *out, &doc)?;
    writeln!(out)
}

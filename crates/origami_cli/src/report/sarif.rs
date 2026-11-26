//! SARIF 2.1.0 output, one rule per diagnostic code.

use super::Run;
use origami_core::{Code, Severity};
use serde_json::{Value, json};
use std::io::Write;

/// `path` as a URI reference (RFC 3986): forward slashes, everything but
/// unreserved characters and `/` percent-encoded, a Windows drive as a
/// `file:` URI. `None` for a pseudo-path such as `<stdin>`.
fn uri(path: &str) -> Option<String> {
    if path.starts_with('<') && path.ends_with('>') {
        return None;
    }
    let path = path.replace('\\', "/");
    let bytes = path.as_bytes();
    let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    let mut out = String::from(if drive { "file:///" } else { "" });
    for (i, &b) in bytes.iter().enumerate() {
        if b.is_ascii_alphanumeric()
            || matches!(b, b'-' | b'.' | b'_' | b'~' | b'/')
            || (drive && i == 1)
        {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    Some(out)
}

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
            let mut result = json!({
                "ruleId": d.code.as_str(),
                "level": level,
                "message": { "text": d.message },
            });
            // A pseudo-path (`<args>`, `<stdin>`, `<internal>`) names no file.
            if let Some(uri) = uri(&f.path) {
                result["locations"] = json!([{
                    "physicalLocation": {
                        "artifactLocation": { "uri": uri },
                        "region": {
                            "startLine": d.range.start.line,
                            "startColumn": d.range.start.column,
                            "endLine": d.range.end.line,
                            "endColumn": d.range.end.column
                        }
                    }
                }]);
            }
            results.push(result);
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

#[cfg(test)]
mod tests {
    use super::uri;

    #[test]
    fn uri_encodes_per_rfc_3986() {
        assert_eq!(
            uri("src/zcl_a.clas.abap").as_deref(),
            Some("src/zcl_a.clas.abap")
        );
        assert_eq!(
            uri("my dir/ä#1.abap").as_deref(),
            Some("my%20dir/%C3%A4%231.abap")
        );
        assert_eq!(uri("src\\a.abap").as_deref(), Some("src/a.abap"));
        assert_eq!(
            uri("C:\\p\\a b.abap").as_deref(),
            Some("file:///C:/p/a%20b.abap")
        );
        assert_eq!(uri("/tmp/a:b"), Some("/tmp/a%3Ab".into()));
        for pseudo in ["<args>", "<stdin>", "<internal>"] {
            assert_eq!(uri(pseudo), None);
        }
    }
}

//! JSON diagnostics, schema version 1 (`schemas/diagnostics.v1.json`).

use super::Run;
use origami_core::{Edition, Reported};
use serde::Serialize;
use std::io::Write;

#[derive(Serialize)]
struct Doc<'a> {
    schema: u32,
    version: &'static str,
    style_edition: &'static str,
    truncated: bool,
    files: Vec<File<'a>>,
}

#[derive(Serialize)]
struct File<'a> {
    path: &'a str,
    language: Option<&'static str>,
    changed: bool,
    status: origami_core::Status,
    diagnostics: &'a [Reported],
}

pub fn write(run: &Run, out: &mut dyn Write) -> std::io::Result<()> {
    let (files, truncated) = run.capped();
    let doc = Doc {
        schema: 1,
        version: env!("CARGO_PKG_VERSION"),
        style_edition: Edition::E2026.as_str(),
        truncated,
        files: files
            .into_iter()
            .map(|(f, diagnostics)| File {
                path: &f.path,
                language: f.language.map(|l| l.as_str()),
                changed: f.changed,
                status: f.status,
                diagnostics,
            })
            .collect(),
    };
    serde_json::to_writer(&mut *out, &doc)?;
    writeln!(out)
}

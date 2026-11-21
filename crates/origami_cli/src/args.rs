//! Command-line arguments (see `docs/CLI.md`).

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DiagFormat {
    Human,
    Json,
    Sarif,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CaseArg {
    Upper,
    Lower,
}

#[derive(Debug, Parser)]
#[command(
    name = "origami",
    about = "Layout-only formatter for ABAP and the CDS family (DDLS, DDLX, DCL, BDEF, SRVD)",
    disable_version_flag = true
)]
pub struct Args {
    /// Files or directories. Directories are walked, honoring .gitignore and
    /// .origamiignore. Default: the current directory.
    pub paths: Vec<PathBuf>,

    /// Read one source from stdin and write the result to stdout.
    #[arg(long)]
    pub stdin: bool,

    /// Name used for language detection and config lookup; implies --stdin.
    #[arg(long, value_name = "PATH")]
    pub stdin_filepath: Option<String>,

    /// abap | ddls | ddlx | dcl | bdef | srvd | cds (CDS family, detected from content).
    #[arg(long, value_name = "LANG")]
    pub language: Option<String>,

    /// Rewrite files in place (default for paths).
    #[arg(long)]
    pub write: bool,

    /// Write nothing; exit 1 if any file would change.
    #[arg(long)]
    pub check: bool,

    /// With --check: print a unified diff of the changes to stdout.
    #[arg(long, requires = "check")]
    pub diff: bool,

    /// Exit 5 if any warning was emitted.
    #[arg(long)]
    pub strict: bool,

    /// Apply style rules that are still in preview.
    #[arg(long)]
    pub preview: bool,

    /// Diagnostics format; always written to stderr.
    #[arg(long, value_enum, default_value_t = DiagFormat::Human, value_name = "FORMAT")]
    pub diagnostics: DiagFormat,

    /// Cap the number of diagnostics per run.
    #[arg(long, default_value_t = 200, value_name = "N")]
    pub max_diagnostics: usize,

    /// Use only this config file, no discovery.
    #[arg(long, value_name = "FILE", conflicts_with = "no_config")]
    pub config: Option<PathBuf>,

    /// Ignore all config files.
    #[arg(long)]
    pub no_config: bool,

    #[arg(long, value_name = "N")]
    pub line_width: Option<u16>,

    #[arg(long, value_name = "N")]
    pub indent_width: Option<u8>,

    #[arg(long, value_enum, value_name = "CASE")]
    pub keyword_case: Option<CaseArg>,

    /// Worker threads for directory mode (default: CPU count).
    #[arg(short = 'j', long = "jobs", value_name = "N")]
    pub jobs: Option<usize>,

    /// Print version and style edition.
    #[arg(long)]
    pub version: bool,

    /// With --version: print JSON.
    #[arg(long, requires = "version")]
    pub json: bool,
}

impl Args {
    pub fn stdin_mode(&self) -> bool {
        self.stdin || self.stdin_filepath.is_some()
    }
}

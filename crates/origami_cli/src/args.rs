//! Command-line arguments (see `docs/CLI.md`).

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DiagFormat {
    Human,
    Json,
    Sarif,
    /// Compact lines for AI agents: findings with fix hints, a NEXT step.
    Agent,
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

    /// Show the changes origami would make, with notes on introduced line
    /// breaks; writes nothing (implies --check).
    #[arg(long, conflicts_with_all = ["diff", "write"])]
    pub review: bool,

    /// Exit 5 if any warning was emitted.
    #[arg(long)]
    pub strict: bool,

    /// Format only changes that touch these lines, START:END (1-based,
    /// inclusive; repeatable). Needs stdin mode or exactly one file.
    #[arg(long, value_name = "START:END", conflicts_with = "changed")]
    pub lines: Vec<String>,

    /// Format only lines changed against a git revision (default HEAD), plus
    /// untracked files in full. The next argument is taken as the revision
    /// unless it starts with `-`; to pass paths, write `--changed=REF path…`
    /// or put the paths first.
    #[arg(long, value_name = "REF", num_args = 0..=1, default_missing_value = "HEAD")]
    pub changed: Option<String>,

    /// Apply style rules that are still in preview.
    #[arg(long)]
    pub preview: bool,

    /// Diagnostics format; always written to stderr. Default: human, or
    /// agent when run by an AI agent (CLAUDECODE is set).
    #[arg(long, value_enum, value_name = "FORMAT", env = "ORIGAMI_DIAGNOSTICS")]
    pub diagnostics: Option<DiagFormat>,

    /// Cap the number of diagnostics per run.
    #[arg(long, default_value_t = 200, value_name = "N")]
    pub max_diagnostics: usize,

    /// Use only this config file, no discovery.
    #[arg(long, value_name = "FILE", conflicts_with = "no_config")]
    pub config: Option<PathBuf>,

    /// Ignore all config files.
    #[arg(long)]
    pub no_config: bool,

    /// Maximum line width, 40..=255 (default 120)
    #[arg(long, value_name = "N")]
    pub line_width: Option<u16>,

    /// Spaces per indent level, 1..=8 (default 2)
    #[arg(long, value_name = "N")]
    pub indent_width: Option<u8>,

    /// Case of ABAP keywords (default upper)
    #[arg(long, value_enum, value_name = "CASE")]
    pub keyword_case: Option<CaseArg>,

    /// Worker threads for directory mode (default: CPU count).
    #[arg(short = 'j', long = "jobs", value_name = "N")]
    pub jobs: Option<usize>,

    /// Run as an MCP server on stdin/stdout (tools `format`, `format_files`).
    #[arg(long)]
    pub mcp: bool,

    /// Print version and style edition.
    #[arg(long)]
    pub version: bool,

    /// With --version: print JSON.
    #[arg(long, requires = "version")]
    pub json: bool,
}

impl Args {
    /// The diagnostics format: as given, else agent output for AI agents,
    /// else human output.
    pub fn diag_format(&self) -> DiagFormat {
        self.diagnostics.unwrap_or_else(|| {
            if std::env::var_os("CLAUDECODE").is_some() {
                DiagFormat::Agent
            } else {
                DiagFormat::Human
            }
        })
    }

    /// Write nothing and report what would change: `--check` or `--review`.
    pub fn check_mode(&self) -> bool {
        self.check || self.review
    }

    pub fn stdin_mode(&self) -> bool {
        self.stdin || self.stdin_filepath.is_some()
    }
}

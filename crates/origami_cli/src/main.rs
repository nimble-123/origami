//! `origami`: layout-only formatter for ABAP and the CDS family.

mod args;
mod changed;
mod config;
mod io;
mod mcp;
mod panic;
mod report;
mod run;
mod walk;

use args::DiagFormat;
use clap::error::ErrorKind;
use clap::{Parser, ValueEnum};

fn main() {
    panic::install_hook();
    let args = match args::Args::try_parse() {
        Ok(a) => a,
        Err(e) => {
            if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                let _ = e.print();
                std::process::exit(0);
            }
            match requested_format() {
                Some(format) if format != DiagFormat::Human => {
                    run::report_usage_error(format, &clap_message(&e));
                }
                _ => {
                    let _ = e.print();
                }
            }
            std::process::exit(3);
        }
    };
    panic::set_quiet(
        !args.mcp && matches!(args.diag_format(), DiagFormat::Json | DiagFormat::Sarif),
    );
    let code = panic::catch(|| run::run(&args))
        .unwrap_or_else(|msg| panic::catch(|| run::internal_error(&args, &msg)).unwrap_or(70));
    std::process::exit(code);
}

/// The diagnostics format asked for on a command line that clap rejected:
/// `--diagnostics F` or `--diagnostics=F`, else the environment.
fn requested_format() -> Option<DiagFormat> {
    let parse = |v: &str| DiagFormat::from_str(v, false).ok();
    let argv: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .take_while(|a| a != "--")
        .collect();
    for (i, a) in argv.iter().enumerate() {
        if let Some(v) = a.strip_prefix("--diagnostics=") {
            return parse(v);
        }
        if a == "--diagnostics" {
            return argv.get(i + 1).and_then(|v| parse(v));
        }
    }
    match std::env::var("ORIGAMI_DIAGNOSTICS") {
        Ok(v) => parse(&v),
        Err(_) if std::env::var_os("CLAUDECODE").is_some() => Some(DiagFormat::Agent),
        Err(_) => None,
    }
}

/// clap's error without the usage block, on one line.
fn clap_message(e: &clap::Error) -> String {
    e.render()
        .to_string()
        .lines()
        .map(str::trim)
        .take_while(|l| !l.starts_with("Usage:") && !l.starts_with("For more information"))
        .filter(|l| !l.is_empty())
        .map(|l| l.strip_prefix("error: ").unwrap_or(l))
        .collect::<Vec<_>>()
        .join("; ")
}

//! `cargo xtask <task>`: corpus fetch/run/bless, explain, freq, oracle, bench-check,
//! release-diff. Run from anywhere inside the repository.

mod bench_check;
mod bench_matrix;
mod corpus;
mod explain;
mod freq;
mod idempotency;
mod oracle;
mod release_diff;
mod stress;

use std::path::PathBuf;
use std::process::ExitCode;

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match a.as_slice() {
        ["corpus", "fetch"] => corpus::fetch(),
        ["corpus", "run"] => corpus::run(false),
        ["corpus", "bless"] => corpus::run(true),
        ["corpus", "run", "--hashes", file] => corpus::run(false).and_then(|()| corpus::hashes(file)),
        ["explain", "--corpus"] => explain::corpus(),
        ["explain", file] => explain::file(file),
        ["freq"] => freq::run(40),
        ["freq", n] => freq::run(n.parse().unwrap_or(40)),
        ["oracle"] => oracle::run(),
        ["bench-check", rest @ ..] => bench_check::run(rest),
        ["bench-matrix"] => bench_matrix::run(),
        ["bench-matrix", "--report"] => bench_matrix::report_only(),
        ["release-diff", rest @ ..] => release_diff::run(rest),
        ["stress", rest @ ..] => stress::run(rest),
        ["idempotency"] => idempotency::run(),
        _ => Err(
            "usage: cargo xtask <corpus fetch|corpus run [--hashes FILE]|corpus bless|explain FILE|explain --corpus|freq [N]|oracle|bench-check [hyperfine.json...]|bench-matrix [--report]|release-diff [tag]|stress [n] [seed]|idempotency>"
                .into(),
        ),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}

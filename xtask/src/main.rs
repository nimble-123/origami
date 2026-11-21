//! `cargo xtask <task>`: corpus fetch/run/bless, freq, oracle, bench-check,
//! release-diff. Run from anywhere inside the repository.

mod bench_check;
mod corpus;
mod freq;
mod oracle;
mod release_diff;

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
        ["freq"] => freq::run(40),
        ["freq", n] => freq::run(n.parse().unwrap_or(40)),
        ["oracle"] => oracle::run(),
        ["bench-check", rest @ ..] => bench_check::run(rest),
        ["release-diff", rest @ ..] => release_diff::run(rest),
        _ => Err(
            "usage: cargo xtask <corpus fetch|corpus run|corpus bless|freq [N]|oracle|bench-check [hyperfine.json...]|release-diff [tag]>"
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

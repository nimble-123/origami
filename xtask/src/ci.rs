//! `ci [--full]`: the local pre-PR checks, in order, stopping at the first
//! failure. `--full` adds the idempotency sweep (minutes).

use crate::root;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

enum Step {
    /// `cargo <args>`
    Cargo(&'static str, &'static [&'static str]),
    /// another xtask subcommand, run with this binary
    Xtask(&'static str, &'static [&'static str]),
}

const STEPS: &[Step] = &[
    Step::Cargo("fmt --check", &["fmt", "--all", "--check"]),
    Step::Cargo(
        "clippy",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    ),
    Step::Cargo("test", &["test", "--workspace"]),
    Step::Xtask("corpus run", &["corpus", "run"]),
    Step::Xtask("explain --corpus", &["explain", "--corpus"]),
    Step::Xtask("oracle", &["oracle"]),
    Step::Xtask("stress 20000 1", &["stress", "20000", "1"]),
];

pub fn run(args: &[&str]) -> Result<(), String> {
    let full = match args {
        [] => false,
        ["--full"] => true,
        _ => return Err("usage: cargo xtask ci [--full]".into()),
    };
    let root = root();
    let xtask = std::env::current_exe().map_err(|e| e.to_string())?;
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let start = Instant::now();
    let mut skipped = Vec::new();

    let mut steps: Vec<(&str, Command)> = Vec::new();
    for step in STEPS {
        let (name, mut cmd) = match step {
            Step::Cargo(name, args) => {
                let mut c = Command::new(&cargo);
                c.args(*args);
                (*name, c)
            }
            Step::Xtask(name, args) => {
                if *name == "oracle" && !oracle_ready(&root) {
                    skipped.push("oracle (run `npm ci` in tools/abaplint-oracle)");
                    continue;
                }
                let mut c = Command::new(&xtask);
                c.args(*args);
                (*name, c)
            }
        };
        cmd.current_dir(&root);
        steps.push((name, cmd));
    }
    if full {
        let mut c = Command::new(&xtask);
        c.arg("idempotency").current_dir(&root);
        steps.push(("idempotency", c));
    } else {
        skipped.push("idempotency (add --full)");
    }

    let total = steps.len();
    for (i, (name, mut cmd)) in steps.into_iter().enumerate() {
        eprintln!("\n==> [{}/{}] {name}", i + 1, total);
        let t = Instant::now();
        let status = cmd.status().map_err(|e| format!("{name}: {e}"))?;
        if !status.success() {
            return Err(format!("{name} failed ({status}); stopped"));
        }
        eprintln!("    ok ({:.0?})", t.elapsed());
    }
    for s in &skipped {
        eprintln!("skipped: {s}");
    }
    eprintln!("\nci: all {total} steps passed in {:.0?}", start.elapsed());
    Ok(())
}

fn oracle_ready(root: &Path) -> bool {
    root.join("tools/abaplint-oracle/node_modules").is_dir()
}

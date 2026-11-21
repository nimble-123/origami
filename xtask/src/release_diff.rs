//! `release-diff [tag]`: downloads the previous release binary with
//! `gh release download`, formats a copy of the corpus with it and with the
//! current release build, and writes `target/release-diff.patch` for the
//! release notes. Needs a published release; until the first one, it reports
//! that there is nothing to compare.

use crate::corpus::{files, manifest};
use crate::root;
use std::path::Path;
use std::process::Command;

fn copy_corpus(to: &Path) -> Result<(), String> {
    let m = manifest()?;
    for r in &m.repo {
        for (rel, path) in files(r) {
            let dest = to.join(&r.name).join(&rel);
            std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::copy(&path, &dest).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn run(args: &[&str]) -> Result<(), String> {
    let tag = args.first().copied();
    let work = root().join("target/release-diff");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(work.join("bin")).map_err(|e| e.to_string())?;

    let mut gh = Command::new("gh");
    gh.args(["release", "download"]);
    if let Some(t) = tag {
        gh.arg(t);
    }
    gh.args(["--pattern", "origami-*", "--dir"])
        .arg(work.join("bin"));
    let status = gh.status().map_err(|e| format!("cannot run gh: {e}"))?;
    if !status.success() {
        println!("release-diff: no previous release to compare with");
        return Ok(());
    }
    let previous = std::fs::read_dir(work.join("bin"))
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .find(|p| !p.to_string_lossy().ends_with(".sha256"))
        .ok_or("downloaded release contains no binary")?;

    let status = Command::new("cargo")
        .args(["build", "--release", "-p", "abap-origami"])
        .current_dir(root())
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("release build failed".into());
    }
    let current = root().join("target/release/origami");

    for (name, bin) in [
        ("previous", previous.as_path()),
        ("current", current.as_path()),
    ] {
        let dir = work.join(name);
        copy_corpus(&dir)?;
        let _ = Command::new(bin).arg("--no-config").arg(&dir).status();
    }
    let out = Command::new("git")
        .args(["diff", "--no-index", "--"])
        .arg(work.join("previous"))
        .arg(work.join("current"))
        .output()
        .map_err(|e| e.to_string())?;
    std::fs::write(root().join("target/release-diff.patch"), &out.stdout)
        .map_err(|e| e.to_string())?;
    println!(
        "release-diff: {} bytes written to target/release-diff.patch",
        out.stdout.len()
    );
    Ok(())
}

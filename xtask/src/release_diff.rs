//! `release-diff [tag]`: downloads the previous release binary with
//! `gh release download`, formats a copy of the corpus with it and with the
//! current release build, and writes `target/release-diff.patch` for the
//! release notes. Without a tag it compares with the latest release; if that
//! has no binary for this machine it reports that there is nothing to
//! compare. With a tag, a failed download is an error.

use crate::corpus::{files, manifest};
use crate::root;
use std::path::Path;
use std::process::Command;

/// Target triple of the release binary that runs on this machine.
fn host_triple() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        (os, arch) => Err(format!("no release binary for {os}/{arch}")),
    }
}

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
    // Only this machine's binary: the release carries one per target, and a
    // binary for another platform cannot run here.
    gh.args([
        "--pattern",
        &format!("origami-*-{}", host_triple()?),
        "--dir",
    ])
    .arg(work.join("bin"));
    let status = gh.status().map_err(|e| format!("cannot run gh: {e}"))?;
    if !status.success() {
        if let Some(t) = tag {
            return Err(format!("cannot download the binary of release {t}"));
        }
        println!("release-diff: no previous release to compare with");
        return Ok(());
    }
    let previous = std::fs::read_dir(work.join("bin"))
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .find(|p| !p.to_string_lossy().ends_with(".sha256"))
        .ok_or("downloaded release contains no binary")?;
    // Downloaded files are not executable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&previous, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }

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
        // The exit status only says whether something changed or failed to
        // format; the diff below is the result. A binary that cannot start
        // is an error.
        Command::new(bin)
            .arg("--no-config")
            .arg(&dir)
            .status()
            .map_err(|e| format!("cannot run {}: {e}", bin.display()))?;
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

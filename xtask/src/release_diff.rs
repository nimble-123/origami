//! `release-diff [tag]`: downloads the previous release binary with
//! `gh release download`, formats a copy of the corpus with it and with the
//! current release build, and writes `target/release-diff.patch` for the
//! release notes. Without a tag it compares with the latest release; if that
//! has no binary for this machine it reports that there is nothing to
//! compare. With a tag, a failed download is an error. The binary is checked
//! against its `.sha256` file, and against its build provenance attestation
//! when it has one, before it is run.

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

/// Checks the downloaded binary against its `.sha256` file and, when `gh` can
/// find the repository, against its build provenance attestation. A binary
/// without an attestation (releases before provenance) only gets a notice; a
/// binary whose attestation does not verify is an error.
fn verify(bin: &Path) -> Result<(), String> {
    let dir = bin.parent().ok_or("binary has no directory")?;
    let name = bin
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("binary has no name")?;
    let sum_file = dir.join(format!("{name}.sha256"));
    let sum = std::fs::read_to_string(&sum_file)
        .map_err(|e| format!("release has no checksum file for {name}: {e}"))?;
    let digest = sum
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string();
    // The file names the binary relative to its own directory.
    let checked = ["sha256sum", "shasum"].iter().find_map(|tool| {
        let mut c = Command::new(tool);
        if *tool == "shasum" {
            c.args(["-a", "256"]);
        }
        c.arg("-c")
            .arg(format!("{name}.sha256"))
            .current_dir(dir)
            .status()
            .ok()
    });
    match checked {
        Some(s) if s.success() => {}
        Some(_) => return Err(format!("checksum of {name} does not match its .sha256")),
        None => return Err("neither sha256sum nor shasum is available".into()),
    }

    let repo = std::env::var("GH_REPO")
        .or_else(|_| std::env::var("GITHUB_REPOSITORY"))
        .unwrap_or_default();
    if repo.is_empty() {
        println!("release-diff: no GH_REPO; build provenance of {name} not checked");
        return Ok(());
    }
    let verified = Command::new("gh")
        .args(["attestation", "verify"])
        .arg(bin)
        .args(["--repo", &repo, "--signer-workflow"])
        .arg(format!("{repo}/.github/workflows/release.yml"))
        .status()
        .map_err(|e| format!("cannot run gh: {e}"))?;
    if verified.success() {
        return Ok(());
    }
    // Releases before provenance have no attestation. One that has an
    // attestation which does not verify must not be run.
    let count = Command::new("gh")
        .args(["api", &format!("repos/{repo}/attestations/sha256:{digest}")])
        .args(["--jq", ".attestations | length"])
        .output()
        .map_err(|e| format!("cannot run gh: {e}"))?;
    let none = if count.status.success() {
        String::from_utf8_lossy(&count.stdout).trim() == "0"
    } else {
        String::from_utf8_lossy(&count.stderr).contains("404")
    };
    if none {
        println!(
            "release-diff: {name} has no build provenance attestation; checksum verified only"
        );
        Ok(())
    } else {
        Err(format!("build provenance of {name} could not be verified"))
    }
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
    // Only this machine's binary and its checksum: the release carries one
    // per target, and a binary for another platform cannot run here.
    let name_pattern = format!("origami-*-{}", host_triple()?);
    gh.args(["--pattern", &name_pattern, "--pattern"])
        .arg(format!("{name_pattern}.sha256"))
        .arg("--dir")
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
    verify(&previous)?;
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

#[cfg(test)]
mod tests {
    use super::verify;

    #[test]
    fn a_wrong_checksum_is_refused() {
        let dir = std::env::temp_dir().join(format!("origami-verify-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bin = dir.join("origami-0.0.0-test");
        std::fs::write(&bin, "binary").unwrap();
        std::fs::write(
            dir.join("origami-0.0.0-test.sha256"),
            format!("{}  origami-0.0.0-test\n", "0".repeat(64)),
        )
        .unwrap();
        let err = verify(&bin).unwrap_err();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(err.contains("does not match"), "{err}");
    }

    #[test]
    fn a_missing_checksum_file_is_refused() {
        let dir = std::env::temp_dir().join(format!("origami-verify-none-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bin = dir.join("origami-0.0.0-test");
        std::fs::write(&bin, "binary").unwrap();
        let err = verify(&bin).unwrap_err();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(err.contains("no checksum file"), "{err}");
    }
}

//! File discovery. Explicit files are always included (unsupported ones get
//! E011 later); directories are walked and filtered to supported types.

use ignore::WalkBuilder;
use origami_core::detect::{FileKind, by_file_name};
use std::path::{Path, PathBuf};

pub fn files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in paths {
        if p.is_dir() {
            walk_dir(p, &mut out);
        } else {
            out.push(p.clone());
        }
    }
    out.sort();
    out.dedup();
    out
}

fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    let walker = WalkBuilder::new(dir)
        .hidden(true)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(false)
        .require_git(false)
        .parents(true)
        .add_custom_ignore_filename(".origamiignore")
        .follow_links(false)
        .build();
    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if by_file_name(&name) != FileKind::Unsupported {
            out.push(entry.into_path());
        }
    }
}

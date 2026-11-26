//! Atomic file writes that keep permissions.
//!
//! The new content goes to a temporary file in the same directory, which then
//! replaces the original with a rename, so a reader never sees a half-written
//! file. There is no fsync: like other formatters, origami does not force the
//! data to disk, which keeps write mode fast.

use std::io::Write;
use std::path::Path;

/// Replaces the file `path` names. A symbolic link is followed, so the link
/// stays a link and its target gets the new content.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let path = &path.canonicalize()?;
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let perms = std::fs::metadata(path)?.permissions();
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(bytes)?;
    std::fs::set_permissions(tmp.path(), perms)?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

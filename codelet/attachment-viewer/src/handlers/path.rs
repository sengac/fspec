//! Lexical path normalization + directory-traversal guard.
//!
//! Port of the TS `validatePath`: resolve the requested path relative to `cwd`,
//! normalize `.`/`..` lexically (WITHOUT requiring the file to exist, so a
//! missing-and-traversing path is rejected as a traversal rather than a 404),
//! and reject anything that escapes `cwd`.

use std::path::{Component, Path, PathBuf};

/// Lexically normalize `path`, folding `.` and `..` components without touching
/// the filesystem. A leading `..` that would escape the root is preserved in the
/// output (so callers can detect escape via a `starts_with` check).
fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out: Vec<Component> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => match out.last() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir) | Some(Component::Prefix(_)) => {}
                _ => out.push(comp),
            },
            other => out.push(other),
        }
    }
    out.iter().collect()
}

/// Resolve `requested` against `cwd` and confirm it stays within `cwd`.
///
/// Returns the normalized absolute path on success, or `None` if the resolved
/// path escapes `cwd` (directory traversal).
pub fn validate_path(cwd: &Path, requested: &str) -> Option<PathBuf> {
    let req = Path::new(requested);
    let joined = if req.is_absolute() {
        req.to_path_buf()
    } else {
        cwd.join(req)
    };

    let normalized = lexical_normalize(&joined);
    let normalized_cwd = lexical_normalize(cwd);

    if normalized.starts_with(&normalized_cwd) {
        Some(normalized)
    } else {
        None
    }
}

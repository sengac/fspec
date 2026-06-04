//! Project root + spec directory detection.
//!
//! Rust port of `src/utils/project-root-detection.ts` (see RPC-253 AST
//! research note for the line-by-line mapping). Walks upward from a starting
//! directory looking for project-boundary markers; falls back to creating
//! `spec/` at the start directory.

use std::path::{Path, PathBuf};

use crate::error::FspecCoreError;

/// Files / directories that mark a project root. Mirrors the TS
/// `BOUNDARY_MARKERS` constant.
const BOUNDARY_MARKERS: &[&str] = &[
    ".git",
    "package.json",
    ".gitignore",
    "Cargo.toml",
    "pyproject.toml",
];

/// Maximum directories to walk upward. Matches TS `MAX_SEARCH_DEPTH = 10`.
const MAX_SEARCH_DEPTH: u32 = 10;

/// Finds or creates the `spec/` directory at the appropriate project root.
///
/// Algorithm (mirrors `findOrCreateSpecDirectory` in the TS source):
/// 1. If `cwd/spec` already exists → use it (test isolation).
/// 2. Walk upward looking for an existing `spec/` that's within a project boundary.
/// 3. Otherwise find the project root via boundary markers and create `spec/` there.
/// 4. On any error: fall back to creating `cwd/spec`.
///
/// All filesystem failures are coerced into the graceful-fallback path —
/// this matches the TS behaviour and keeps the dispatcher resilient when
/// the agent is sandboxed without filesystem privileges above `cwd`.
pub fn find_or_create_spec_directory(cwd: &Path) -> Result<PathBuf, FspecCoreError> {
    // Safety check: if spec/ already exists at cwd, use it (test isolation).
    let cwd_spec = cwd.join("spec");
    if cwd_spec.is_dir() {
        return Ok(cwd_spec);
    }

    // First, try to find an existing spec/ within the project boundary.
    if let Some(existing) = find_existing_spec_directory(cwd) {
        return Ok(existing);
    }

    // Otherwise find the project root and create spec/ there.
    let project_root = find_project_root(cwd);
    let spec_path = project_root.join("spec");
    match std::fs::create_dir_all(&spec_path) {
        Ok(()) => Ok(spec_path),
        Err(_) => {
            // Graceful fallback: create spec/ at cwd.
            let fallback = cwd.join("spec");
            std::fs::create_dir_all(&fallback).map_err(|source| FspecCoreError::Io {
                command: "find_or_create_spec_directory",
                source,
            })?;
            Ok(fallback)
        }
    }
}

/// Searches upward for an existing `spec/` directory inside a project boundary.
fn find_existing_spec_directory(cwd: &Path) -> Option<PathBuf> {
    let mut current = cwd.to_path_buf();
    let mut depth = 0u32;

    while depth < MAX_SEARCH_DEPTH {
        let candidate = current.join("spec");
        if candidate.is_dir() && has_project_boundary_marker(&current) {
            return Some(candidate);
        }

        match current.parent() {
            Some(parent) if parent != current => {
                current = parent.to_path_buf();
                depth += 1;
            }
            _ => break,
        }
    }

    None
}

/// Walks upward for the nearest directory containing any boundary marker.
/// Returns `cwd` if nothing found (matches TS fallback).
pub fn find_project_root(cwd: &Path) -> PathBuf {
    let mut current = cwd.to_path_buf();
    let mut depth = 0u32;

    while depth < MAX_SEARCH_DEPTH {
        if has_project_boundary_marker(&current) {
            return current;
        }

        match current.parent() {
            Some(parent) if parent != current => {
                current = parent.to_path_buf();
                depth += 1;
            }
            _ => break,
        }
    }

    cwd.to_path_buf()
}

/// Returns true if `dir` contains any boundary marker file/directory.
fn has_project_boundary_marker(dir: &Path) -> bool {
    BOUNDARY_MARKERS
        .iter()
        .any(|marker| dir.join(marker).exists())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn uses_existing_cwd_spec_when_present() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("spec")).unwrap();
        let result = find_or_create_spec_directory(tmp.path()).unwrap();
        assert_eq!(result, tmp.path().join("spec"));
    }

    #[test]
    fn creates_spec_at_cwd_when_no_marker_anywhere() {
        let tmp = TempDir::new().unwrap();
        // No boundary markers anywhere — find_project_root falls back to cwd,
        // and we create spec/ there.
        let result = find_or_create_spec_directory(tmp.path()).unwrap();
        assert!(result.is_dir());
        assert_eq!(result.parent().unwrap(), tmp.path());
    }

    #[test]
    fn creates_spec_at_marker_root_when_found_via_parents() {
        let tmp = TempDir::new().unwrap();
        // Create a marker at the root, then a nested cwd without spec/.
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname=\"x\"").unwrap();
        let nested = tmp.path().join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();
        let result = find_or_create_spec_directory(&nested).unwrap();
        // Should create spec/ at the marker root, not at nested.
        assert_eq!(result, tmp.path().join("spec"));
        assert!(result.is_dir());
    }
}

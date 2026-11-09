//! Project root + spec directory detection.
//!
//! Rust port of `src/utils/project-root-detection.ts` (see RPC-253 AST
//! research note for the line-by-line mapping). Walks upward from a starting
//! directory looking for project-boundary markers; falls back to creating
//! `spec/` at the start directory.

use std::path::{Path, PathBuf};

// BUG-497: the resolution lives in codelet-common so the config code and
// codelet-core share it; fspec-core keeps re-exporting it.
pub use codelet_common::project_root::{
    find_existing_spec_directory, find_project_root, resolve_project_root,
};

use crate::error::FspecCoreError;

/// Finds or creates the `spec/` directory at the appropriate project root.
///
/// Algorithm (mirrors `findOrCreateSpecDirectory` in the TS source; steps 1-3
/// are [`resolve_project_root`]):
/// 1. If `cwd/spec` already exists → use it (test isolation).
/// 2. Walk upward looking for an existing `spec/` that's within a project boundary.
/// 3. Otherwise find the project root via boundary markers and create `spec/` there.
/// 4. On any error: fall back to creating `cwd/spec`.
///
/// All filesystem failures are coerced into the graceful-fallback path —
/// this matches the TS behaviour and keeps the dispatcher resilient when
/// the agent is sandboxed without filesystem privileges above `cwd`.
pub fn find_or_create_spec_directory(cwd: &Path) -> Result<PathBuf, FspecCoreError> {
    let spec_path = crate::io::spec_paths::spec_dir_of(&resolve_project_root(cwd));
    if spec_path.is_dir() {
        return Ok(spec_path);
    }

    // No spec/ yet: create it at the resolved project root.
    match std::fs::create_dir_all(&spec_path) {
        Ok(()) => Ok(spec_path),
        Err(_) => {
            // Graceful fallback: create spec/ at cwd.
            let fallback = crate::io::spec_paths::spec_dir_of(cwd);
            std::fs::create_dir_all(&fallback).map_err(|source| FspecCoreError::Io {
                command: "find_or_create_spec_directory",
                source,
            })?;
            Ok(fallback)
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
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

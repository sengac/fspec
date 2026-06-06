//! Shared helper that enumerates `spec/features/**/*.feature` for the
//! project rooted at the supplied `project_root`. Returns relative paths
//! using forward-slash separators (parity with TypeScript tinyglobby,
//! which normalises Windows backslashes to `/`) sorted in alphabetical
//! order so callers can render deterministic output without re-sorting.
//!
//! RPC-245 introduces this helper for `list-features`, but the function
//! is intentionally generic — any future gherkin-aware command (e.g.
//! `validate`, `format`, `show-feature`) will reuse it instead of
//! re-implementing the walk.
//!
//! Error semantics: missing `spec/features/` directory escalates as
//! [`FspecCoreError::DirectoryNotFound { path: "spec/features/" }`]
//! whose `Display` contains the exact substring
//! `"Directory not found: spec/features/"` (parity with TS
//! `src/commands/list-features.ts:33-38`). The dedicated
//! `DirectoryNotFound` variant is returned directly (see lines 34-36);
//! no intermediate `InvalidArgs` substring sniffing is required at the
//! call sites.

use std::path::{Path, PathBuf};

use crate::error::FspecCoreError;

/// Walk `<project_root>/spec/features/` recursively and return every
/// `*.feature` file path relative to `project_root` with forward-slash
/// separators, sorted alphabetically.
///
/// Returns `Err(FspecCoreError::DirectoryNotFound { path: "spec/features/" })`
/// when the directory does not exist (parity with TS `access(featuresDir)`
/// ENOENT branch). The `Display` impl produces the canonical
/// `"Directory not found: spec/features/"` substring.
pub fn glob_feature_files(project_root: &Path) -> Result<Vec<String>, FspecCoreError> {
    let features_dir = project_root.join("spec").join("features");
    if !features_dir.exists() {
        return Err(FspecCoreError::DirectoryNotFound {
            path: "spec/features/".to_string(),
        });
    }

    let mut out: Vec<String> = Vec::new();
    walk(&features_dir, project_root, &mut out)?;
    out.sort();
    Ok(out)
}

/// Recursive helper. Pushes every `*.feature` file under `dir` into
/// `out` as a forward-slash relative path against `project_root`.
fn walk(dir: &Path, project_root: &Path, out: &mut Vec<String>) -> Result<(), FspecCoreError> {
    let entries = std::fs::read_dir(dir).map_err(|source| FspecCoreError::Io {
        command: "list-features",
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| FspecCoreError::Io {
            command: "list-features",
            source,
        })?;
        let path: PathBuf = entry.path();
        let file_type = entry.file_type().map_err(|source| FspecCoreError::Io {
            command: "list-features",
            source,
        })?;
        if file_type.is_dir() {
            walk(&path, project_root, out)?;
        } else if file_type.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some("feature")
        {
            let rel = path
                .strip_prefix(project_root)
                .map_err(|_| FspecCoreError::InvalidArgs {
                    command: "list-features",
                    reason: format!("path not under project root: {}", path.display()),
                })?;
            // tinyglobby normalises path separators to '/' on every
            // platform; the Rust port must match byte-for-byte so the
            // alphabetical sort and downstream rendering are stable.
            let normalised = rel.to_string_lossy().replace('\\', "/");
            out.push(normalised);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn missing_features_dir_returns_directory_not_found() {
        let tmp = TempDir::new().unwrap();
        let err = glob_feature_files(tmp.path()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Directory not found: spec/features/"),
            "expected canonical substring; got: {msg}"
        );
    }

    #[test]
    fn empty_features_dir_returns_empty_list() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("spec/features")).unwrap();
        let files = glob_feature_files(tmp.path()).unwrap();
        assert!(files.is_empty(), "expected empty list, got: {files:?}");
    }

    #[test]
    fn walks_nested_directories_and_sorts_alphabetically() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("spec/features")).unwrap();
        write(tmp.path(), "spec/features/zebra.feature", "Feature: Z\n");
        write(tmp.path(), "spec/features/alpha.feature", "Feature: A\n");
        write(tmp.path(), "spec/features/nested/mango.feature", "Feature: M\n");
        let files = glob_feature_files(tmp.path()).unwrap();
        assert_eq!(
            files,
            vec![
                "spec/features/alpha.feature".to_string(),
                "spec/features/nested/mango.feature".to_string(),
                "spec/features/zebra.feature".to_string(),
            ]
        );
    }

    #[test]
    fn non_feature_extensions_are_ignored() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("spec/features")).unwrap();
        write(tmp.path(), "spec/features/a.feature", "Feature: A\n");
        write(tmp.path(), "spec/features/b.feature.coverage", "{}\n");
        write(tmp.path(), "spec/features/README.md", "x\n");
        let files = glob_feature_files(tmp.path()).unwrap();
        assert_eq!(files, vec!["spec/features/a.feature".to_string()]);
    }
}

//! RPC-020 — file search helper.
//!
//! Feature: spec/features/rpc020-source-shape.feature
//! Feature: spec/features/rpc020-cross-transport-parity.feature
//!
//! Used by `codelet_rpc::FspecServiceImpl::search_files` to answer the
//! AgentView's @file popup. Mirrors the existing
//! `codelet_tools::glob::GlobTool::call` algorithm (ignore::WalkBuilder
//! + globset::GlobBuilder, case-insensitive, sort by mtime desc) but
//!   lives in codelet_core so the dep boundary stays clean — neither
//!   codelet_rpc nor codelet_fspec_tui need to depend on codelet_tools
//!   to surface this helper.
//!
//! Designed to be FAST + SAFE for interactive popups:
//!
//! - Respects .gitignore by default (no node_modules / target spam).
//! - Case-insensitive (`@rea` finds `README.md`).
//! - Sorts by modification time descending so freshly-edited files
//!   bubble to the top of the popup.
//! - Capped at `limit` entries so a wide-open prefix doesn't drown
//!   the popup.

use globset::GlobBuilder;
use ignore::WalkBuilder;
use std::path::Path;
use std::time::SystemTime;

/// Search for files whose path matches `**/*<prefix>*` (case-insensitive)
/// under `cwd`. Returns up to `limit` paths (relative to `cwd`) sorted by
/// modification time descending. Empty `prefix` returns the same as a
/// wildcard match — typically NOT what callers want, so the upstream
/// FspecService skips the empty-prefix case.
///
/// Returns an empty Vec on any I/O / pattern error so the popup
/// degrades gracefully (the user sees "no matches" rather than a panic).
pub fn search(cwd: &Path, prefix: &str, limit: u32) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    let pattern = format!("**/*{prefix}*");
    let matcher = match GlobBuilder::new(&pattern)
        .literal_separator(false)
        .case_insensitive(true)
        .build()
    {
        Ok(g) => g.compile_matcher(),
        Err(_) => return Vec::new(),
    };

    if !cwd.is_dir() {
        return Vec::new();
    }

    let mut matches: Vec<(std::path::PathBuf, SystemTime)> = Vec::new();
    for entry in WalkBuilder::new(cwd)
        .hidden(false)
        .git_ignore(true)
        .build()
        .flatten()
    {
        let is_file = entry.file_type().is_some_and(|ft| ft.is_file());
        if !is_file {
            continue;
        }
        let path = entry.path();
        // Match against both absolute path and the relative path under
        // cwd so prefix queries like "src/main" continue to work even
        // when the absolute prefix is irrelevant. globset matches the
        // path as supplied — we hand it the absolute path so `**/*<prefix>*`
        // catches the trailing segment.
        if !matcher.is_match(path) {
            continue;
        }
        let mtime = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        matches.push((path.to_path_buf(), mtime));
    }

    matches.sort_by_key(|b| std::cmp::Reverse(b.1));

    matches
        .into_iter()
        .take(limit as usize)
        .map(|(p, _)| {
            p.strip_prefix(cwd)
                .map(|rel| rel.display().to_string())
                .unwrap_or_else(|_| p.display().to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_workspace_with_files(files: &[&str]) -> TempDir {
        let tmp = TempDir::new().expect("tempdir");
        for f in files {
            let p = tmp.path().join(f);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).expect("mkdir");
            }
            fs::write(&p, b"x").expect("write");
        }
        tmp
    }

    #[test]
    fn search_finds_readme_by_prefix() {
        let tmp = make_workspace_with_files(&["README.md", "src/main.rs", "tests/test1.rs"]);
        let out = search(tmp.path(), "README", 10);
        assert!(
            out.iter().any(|p| p.ends_with("README.md")),
            "expected README.md in results, got: {out:?}"
        );
    }

    #[test]
    fn search_returns_empty_for_no_match() {
        let tmp = make_workspace_with_files(&["README.md"]);
        let out = search(tmp.path(), "zzz-no-such-prefix", 10);
        assert!(out.is_empty(), "expected empty, got: {out:?}");
    }

    #[test]
    fn search_honours_limit_arg() {
        let mut files: Vec<String> = (0..25).map(|i| format!("file{i}.txt")).collect();
        let owned: Vec<&str> = files.iter_mut().map(|s| s.as_str()).collect();
        let tmp = make_workspace_with_files(&owned);
        let out = search(tmp.path(), "file", 5);
        assert_eq!(out.len(), 5, "got: {out:?}");
    }

    #[test]
    fn search_is_case_insensitive() {
        let tmp = make_workspace_with_files(&["README.md"]);
        let out = search(tmp.path(), "readme", 10);
        assert!(
            out.iter().any(|p| p.ends_with("README.md")),
            "case-insensitive search should match: {out:?}"
        );
    }

    #[test]
    fn search_returns_empty_when_cwd_does_not_exist() {
        let out = search(Path::new("/nonexistent/path/zzzz"), "x", 10);
        assert!(out.is_empty());
    }

    #[test]
    fn search_returns_empty_when_limit_is_zero() {
        let tmp = make_workspace_with_files(&["README.md"]);
        let out = search(tmp.path(), "README", 0);
        assert!(out.is_empty());
    }
}

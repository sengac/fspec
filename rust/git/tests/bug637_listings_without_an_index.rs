//! BUG-637: the staged, unstaged and changed-files listings and StagedFiles
//! read a missing index as empty.
//!
//! Feature: spec/features/git-listings-read-a-missing-index-as-empty.feature
//!
//! Every repository is a fresh temp repo; every git call ignores the user's
//! global and system config.

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_git::status::{capture_changed_files, get_staged_files, get_unstaged_files};
use codelet_git::StagedFiles;
use codelet_test_helpers::{TestError, TestResult};

/// A fresh `git init` holding a.txt, with no index file.
fn fresh_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::tempdir()?;
    let out = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()?;
    if !out.status.success() {
        return Err(format!("git init failed: {}", String::from_utf8_lossy(&out.stderr)).into());
    }
    fs::write(dir.path().join("a.txt"), "a\n")?;
    if Path::new(&dir.path().join(".git/index")).exists() {
        return Err("a fresh git init must have no index".into());
    }
    Ok(dir)
}

#[test]
fn listings_in_a_fresh_git_init_with_no_index() -> TestResult {
    // @step Given a fresh git init holding a.txt and no .git/index
    let repo = fresh_repo()?;
    // @step When the staged, unstaged and changed-files listings are read
    let staged = get_staged_files(repo.path())?;
    let unstaged = get_unstaged_files(repo.path())?;
    let changed = capture_changed_files(repo.path())?;
    // @step Then the staged and unstaged listings are empty
    assert_eq!(staged, Vec::<String>::new());
    assert_eq!(unstaged, Vec::<String>::new());
    // @step And the changed-files capture lists only a.txt, untracked, as "A"
    let listed: Vec<(String, String, bool)> = changed
        .into_iter()
        .map(|entry| (entry.path, entry.change_type, entry.staged))
        .collect();
    assert_eq!(listed, vec![("a.txt".to_string(), "A".to_string(), false)]);
    Ok(())
}

#[test]
fn staged_files_opens_in_a_fresh_git_init_with_no_index() -> TestResult {
    // @step Given a fresh git init holding a.txt and no .git/index
    let repo = fresh_repo()?;
    // @step When StagedFiles is opened
    let staged = StagedFiles::open(repo.path())?;
    // @step Then it succeeds and holds no staged blob for a.txt
    assert!(!staged.contains("a.txt"));
    assert_eq!(staged.paths().count(), 0);
    Ok(())
}

#[test]
fn an_unreadable_index_is_still_an_error() -> TestResult {
    // @step Given a repository whose .git/index holds bytes that are not an index
    let repo = fresh_repo()?;
    // 64 bytes with no DIRC header (gix-index 0.40 panics on a file shorter
    // than its trailing checksum, so the garbage is longer than that).
    fs::write(repo.path().join(".git/index"), b"not an index ".repeat(5))?;
    // @step When the staged listing is read
    let result = get_staged_files(repo.path());
    // @step Then it fails with an error
    assert!(result.is_err(), "{result:?}");
    Ok(())
}

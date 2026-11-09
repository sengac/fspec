//! BUG-696: a .git/index shorter than its header and checksum is an error
//! naming the index, never a gix-index panic.
//!
//! Feature: spec/features/a-truncated-git-index-is-an-error-not-a-panic.feature
//!
//! Every repository is a fresh temp repo; every git call ignores the user's
//! global and system config.

use std::fs;
use std::process::Command;

use codelet_git::status::{capture_changed_files, get_staged_files, get_unstaged_files};
use codelet_git::tree_utils::collect_worktree_files;
use codelet_git::{git_add, git_commit, GitError, StagedFiles};
use codelet_test_helpers::{TestError, TestResult};

/// A fresh `git init` holding a.txt whose .git/index holds `index`.
fn repo_with_index(index: &[u8]) -> Result<tempfile::TempDir, TestError> {
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
    fs::write(dir.path().join(".git/index"), index)?;
    Ok(dir)
}

/// The error names the index file and its length.
fn assert_names_truncated_index(what: &str, error: &GitError) {
    let message = error.to_string();
    assert!(
        message.contains(".git/index") && message.contains("5 bytes"),
        "{what}: the error does not name the truncated index and its length: {message}"
    );
}

fn assert_corrupted_index(what: &str, error: &GitError) {
    assert!(
        matches!(error, GitError::CorruptedIndex { .. }),
        "{what}: not a corrupted index error: {error:?}"
    );
    assert_names_truncated_index(what, error);
}

#[test]
fn listings_report_a_truncated_index() -> TestResult {
    // @step Given a repository whose .git/index holds 5 bytes
    let repo = repo_with_index(b"DIRC\0")?;
    // @step When the staged, unstaged and changed-files listings are read and StagedFiles is opened
    let staged = get_staged_files(repo.path()).err();
    let unstaged = get_unstaged_files(repo.path()).err();
    let changed = capture_changed_files(repo.path()).err();
    let opened = StagedFiles::open(repo.path()).err();
    // @step Then each fails with an error naming the index and its 5 bytes
    for (what, error) in [
        ("staged listing", staged),
        ("unstaged listing", unstaged),
        ("changed-files capture", changed),
        ("StagedFiles::open", opened),
    ] {
        let error = error.ok_or(format!("{what} read a truncated index"))?;
        assert_corrupted_index(what, &error);
    }
    Ok(())
}

#[test]
fn the_worktree_snapshot_reports_a_truncated_index() -> TestResult {
    // @step Given a repository whose .git/index holds 5 bytes
    let repo = repo_with_index(b"DIRC\0")?;
    // @step When the worktree snapshot is collected
    let error = collect_worktree_files(repo.path())
        .err()
        .ok_or("the worktree snapshot read a truncated index")?;
    // @step Then it fails with a corrupted index error naming the index
    assert_corrupted_index("worktree snapshot", &error);
    Ok(())
}

#[test]
fn staging_and_committing_report_a_truncated_index() -> TestResult {
    // @step Given a repository whose .git/index holds 5 bytes
    let repo = repo_with_index(b"DIRC\0")?;
    // @step When a file is staged and a commit is made
    let added = git_add(repo.path(), "a.txt").err();
    let committed = git_commit(repo.path(), "message", "Test", "test@example.com").err();
    // @step Then each fails with a corrupted index error naming the index
    for (what, error) in [("git_add", added), ("git_commit", committed)] {
        let error = error.ok_or(format!("{what} read a truncated index"))?;
        assert_corrupted_index(what, &error);
    }
    Ok(())
}

#[test]
fn an_index_of_the_minimum_length_still_goes_to_gix() -> TestResult {
    // @step Given a repository whose .git/index holds 32 bytes that are not an index
    let repo = repo_with_index(&[b'x'; 32])?;
    // @step When the staged listing is read
    let error = get_staged_files(repo.path())
        .err()
        .ok_or("the staged listing read 32 bytes of garbage as an index")?;
    // @step Then it fails with gix's decode error, not the length error
    let message = error.to_string();
    assert!(
        !matches!(error, GitError::CorruptedIndex { .. }) && !message.contains("32 bytes"),
        "a 32-byte index was refused by the length check: {message}"
    );
    Ok(())
}

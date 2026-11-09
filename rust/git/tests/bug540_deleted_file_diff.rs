// Feature: spec/features/rust-changed-files-view.feature
//! BUG-540 — the diff of a deleted file is its full removal, not an error.
//!
//! Real temp git repositories built with real `git` and read with the real
//! gitoxide-backed `get_file_diff`; no mocking.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_git::{get_file_diff, GitError};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?} failed: {output:?}");
}

/// A repository whose one commit holds `config.json` with the lines "a" and "b".
fn repo_with_config_json() -> TempDir {
    let tmp = TempDir::new().expect("temp dir");
    let repo = tmp.path();
    git(repo, &["init", "-q"]);
    git(repo, &["config", "user.email", "test@example.com"]);
    git(repo, &["config", "user.name", "Test User"]);
    fs::write(repo.join("config.json"), "a\nb\n").expect("write config.json");
    git(repo, &["add", "config.json"]);
    git(repo, &["commit", "-q", "-m", "initial"]);
    tmp
}

fn removed_and_added(diff: &str) -> (Vec<&str>, Vec<&str>) {
    let body = diff
        .lines()
        .filter(|line| !line.starts_with("---") && !line.starts_with("+++"));
    let (mut removed, mut added) = (Vec::new(), Vec::new());
    for line in body {
        if let Some(text) = line.strip_prefix('-') {
            removed.push(text);
        } else if let Some(text) = line.strip_prefix('+') {
            added.push(text);
        }
    }
    (removed, added)
}

// Scenario: The diff of a file deleted from the working tree is its full removal
#[test]
fn the_diff_of_a_file_deleted_from_the_working_tree_is_its_full_removal() {
    // @step Given a repository where "config.json" was committed with the lines "a" and "b"
    let tmp = repo_with_config_json();
    let repo = tmp.path();
    // @step And "config.json" was deleted from the working tree
    fs::remove_file(repo.join("config.json")).expect("delete config.json");
    // @step When the diff of "config.json" is read
    let diff = get_file_diff(repo, "config.json")
        .expect("a deleted file has a diff")
        .expect("the diff is not empty");
    let (removed, added) = removed_and_added(&diff);
    // @step Then the diff removes the lines "a" and "b"
    assert_eq!(removed, vec!["a", "b"], "{diff}");
    // @step And the diff adds no line
    assert!(added.is_empty(), "{diff}");
}

// Scenario: The diff of a deleted and staged file is its full removal
#[test]
fn the_diff_of_a_deleted_and_staged_file_is_its_full_removal() {
    // @step Given a repository where "config.json" was committed with the lines "a" and "b"
    let tmp = repo_with_config_json();
    let repo = tmp.path();
    // @step And "config.json" was deleted and the deletion staged
    git(repo, &["rm", "-q", "config.json"]);
    // @step When the diff of "config.json" is read
    let diff = get_file_diff(repo, "config.json")
        .expect("a deleted file has a diff")
        .expect("the diff is not empty");
    let (removed, added) = removed_and_added(&diff);
    // @step Then the diff removes the lines "a" and "b"
    assert_eq!(removed, vec!["a", "b"], "{diff}");
    // @step And the diff adds no line
    assert!(added.is_empty(), "{diff}");
}

// Scenario: The diff of a path in neither HEAD nor the working tree fails
#[test]
fn the_diff_of_a_path_in_neither_head_nor_the_working_tree_fails() {
    // @step Given a repository where "never-existed.txt" was never committed
    let tmp = repo_with_config_json();
    let repo = tmp.path();
    // @step When the diff of "never-existed.txt" is read
    let result = get_file_diff(repo, "never-existed.txt");
    // @step Then reading the diff fails with file not found
    match result {
        Err(GitError::FileNotFound(path)) => assert_eq!(path, "never-existed.txt"),
        other => panic!("expected FileNotFound, got {other:?}"),
    }
}

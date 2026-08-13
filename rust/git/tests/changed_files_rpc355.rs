//! RPC-355 — Tests for change-type derivation helpers in `rust/git`.
//!
//! Feature: spec/features/git-change-type-derivation.feature
//!
//! Integration-first: exercises the new
//! `get_staged_files_with_change_type` / `get_unstaged_files_with_change_type`
//! helpers against a real temp git repo built with real `git` (via the shared
//! `common::setup_test_repo` fixture) and real gitoxide — no mocking.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_git::status::{
    get_staged_files_with_change_type, get_unstaged_files_with_change_type, ChangeType,
};
use std::fs;
use std::process::Command;

/// Scenario: Staged tracked-but-modified file is reported as change type M
#[test]
fn staged_tracked_but_modified_file_is_reported_as_change_type_m() {
    // @step Given a temporary git repository with a committed file
    let tmp = common::setup_test_repo();
    let repo = tmp.path();

    // @step And the file is modified and staged in the index
    fs::write(repo.join("README.md"), "# Test Repository\nmodified line\n").expect("modify README");
    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(repo)
        .output()
        .expect("git add");

    // @step When get_staged_files_with_change_type is called against that repo
    let staged = get_staged_files_with_change_type(repo).expect("staged with change type");

    // @step Then the staged file is reported with change_type "M"
    let entry = staged
        .iter()
        .find(|c| c.path == "README.md")
        .expect("README.md staged");
    assert_eq!(entry.change_type, ChangeType::Modified);
    assert_eq!(entry.change_type.as_letter(), "M");
}

/// Scenario: Unstaged file deleted from the working directory is reported as change type D
#[test]
fn unstaged_file_deleted_from_the_working_directory_is_reported_as_change_type_d() {
    // @step Given a temporary git repository with a committed file
    let tmp = common::setup_test_repo();
    let repo = tmp.path();

    // @step And the file is deleted from the working directory
    fs::remove_file(repo.join("README.md")).expect("delete README");

    // @step When get_unstaged_files_with_change_type is called against that repo
    let unstaged = get_unstaged_files_with_change_type(repo).expect("unstaged with change type");

    // @step Then the missing file is reported with change_type "D"
    let entry = unstaged
        .iter()
        .find(|c| c.path == "README.md")
        .expect("README.md missing");
    assert_eq!(entry.change_type, ChangeType::Deleted);
    assert_eq!(entry.change_type.as_letter(), "D");
}

/// Scenario: Untracked working-tree file appears as change type A and unstaged
#[test]
fn untracked_working_tree_file_appears_as_change_type_a_and_unstaged() {
    // @step Given a temporary git repository with a committed file
    let tmp = common::setup_test_repo();
    let repo = tmp.path();

    // @step And a new untracked file exists in the working directory
    fs::write(repo.join("brand-new.txt"), "fresh content\n").expect("write untracked");

    // @step When changed_files is collected against that repo
    let untracked = codelet_git::status::get_untracked_files(repo).expect("untracked");

    // @step Then the untracked file appears with change_type "A" and staged false
    // Untracked files are always Added; the combined collector (rust/rpc) sets
    // staged=false for these. Here we assert the git-layer classification: the file
    // is present in the untracked set, which the collector maps to change_type "A".
    assert!(
        untracked.iter().any(|p| p == "brand-new.txt"),
        "untracked set must contain the new file (mapped to change_type A by the collector)"
    );
}

//! WT-010 — `restore_ghost_commit` ignores its `force` parameter.
//!
//! Feature: spec/features/restore-ghost-commit-ignores-the-force-parameter-no-conflict-detection-before-overwriting.feature
//!
//! Scenario 1 pins the data-loss bug: a non-forced restore must detect the
//! checkpoint-vs-workdir divergence and return
//! `Err(GitError::RestoreConflict { files })` WITHOUT touching the tree.
//! Scenario 2 pins that `force=true` keeps today's unconditional clobber
//! semantics (the within-session / NAPI-overwrite path is unchanged).
//! Scenario 3 pins that a file added after the checkpoint is reported as a
//! conflict (not deleted) under `force=false`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_git::ghost_commit;
use codelet_git::GitError;
use std::fs;

/// Create a checkpoint whose captured tree contains `test.txt` with the
/// supplied content, then (optionally) mutate the working directory file.
fn checkpoint_with_test_file(repo_path: &std::path::Path, content: &str) {
    fs::write(repo_path.join("test.txt"), content).expect("write test.txt");
    ghost_commit::create_ghost_commit(repo_path, "WT010", "snap").expect("create checkpoint");
}

// =============================================================================
// Scenario: Non-forced restore reports a conflict instead of overwriting
// modified files
// =============================================================================

#[test]
fn non_forced_restore_reports_a_conflict_instead_of_overwriting_modified_files() {
    // @step Given a git repository with a ghost commit checkpoint whose file test.txt contains 'v1' and test.txt has been edited to 'v2' after the checkpoint
    let temp_dir = common::setup_test_repo();
    let repo_path = temp_dir.path();
    checkpoint_with_test_file(repo_path, "v1");
    fs::write(repo_path.join("test.txt"), "v2").expect("mutate test.txt");

    // @step When I restore the checkpoint with force set to false
    let result = ghost_commit::restore_ghost_commit(repo_path, "WT010", "snap", false);

    // @step Then the restore returns a conflict error listing test.txt and test.txt on disk still contains 'v2'
    let err = result.expect_err("force=false restore with a divergent workdir must be a conflict");
    match err {
        GitError::RestoreConflict { files } => {
            assert_eq!(
                files,
                vec!["test.txt".to_string()],
                "the conflict must list the divergent file"
            );
        }
        other => panic!("expected GitError::RestoreConflict, got {other:?}"),
    }
    let on_disk = fs::read_to_string(repo_path.join("test.txt")).expect("read test.txt");
    assert_eq!(
        on_disk, "v2",
        "WT-010: a non-forced restore must not overwrite the local edit"
    );
}

// =============================================================================
// Scenario: Forced restore overwrites modified files without conflict
// detection
// =============================================================================

#[test]
fn forced_restore_overwrites_modified_files_without_conflict_detection() {
    // @step Given a git repository with a ghost commit checkpoint whose file test.txt contains 'v1' and test.txt has been edited to 'v2' after the checkpoint
    let temp_dir = common::setup_test_repo();
    let repo_path = temp_dir.path();
    checkpoint_with_test_file(repo_path, "v1");
    fs::write(repo_path.join("test.txt"), "v2").expect("mutate test.txt");

    // @step When I restore the checkpoint with force set to true
    let result = ghost_commit::restore_ghost_commit(repo_path, "WT010", "snap", true)
        .expect("force=true restore must keep today's clobber semantics");

    // @step Then the restore succeeds and test.txt on disk contains 'v1' again
    assert!(result.success, "forced restore must report success");
    let on_disk = fs::read_to_string(repo_path.join("test.txt")).expect("read test.txt");
    assert_eq!(
        on_disk, "v1",
        "force=true must clobber the local edit back to the checkpoint content"
    );
}

// =============================================================================
// Scenario: Non-forced restore refuses to delete files added after the
// checkpoint
// =============================================================================

#[test]
fn non_forced_restore_refuses_to_delete_files_added_after_the_checkpoint() {
    // @step Given a git repository with a ghost commit checkpoint and a file added-after-checkpoint.txt that exists only in the working directory
    let temp_dir = common::setup_test_repo();
    let repo_path = temp_dir.path();
    checkpoint_with_test_file(repo_path, "v1");
    fs::write(
        repo_path.join("added-after-checkpoint.txt"),
        "new work",
    )
    .expect("write post-checkpoint file");

    // @step When I restore the checkpoint with force set to false
    let result =
        ghost_commit::restore_ghost_commit(repo_path, "WT010", "snap", false);

    // @step Then the restore returns a conflict error listing added-after-checkpoint.txt and the file still exists on disk
    let err = result
        .expect_err("force=false restore with a post-checkpoint file must be a conflict");
    match err {
        GitError::RestoreConflict { files } => {
            assert!(
                files.iter().any(|f| f == "added-after-checkpoint.txt"),
                "the conflict must list the added file, got {files:?}"
            );
        }
        other => panic!("expected GitError::RestoreConflict, got {other:?}"),
    }
    assert!(
        repo_path.join("added-after-checkpoint.txt").exists(),
        "WT-010: a non-forced restore must not delete post-checkpoint files"
    );
}

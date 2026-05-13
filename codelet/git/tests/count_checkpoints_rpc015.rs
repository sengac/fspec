//! RPC-015 — Tests for `codelet_git::ghost_commit::count_checkpoints`.
//!
//! Feature: spec/features/rpc015-count-checkpoints-helper.feature
//!
//! Pure-helper tests — exercises the new `count_checkpoints` aggregate
//! against fixture git repos with hand-constructed checkpoint refs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_git::ghost_commit;
use codelet_rpc_types::CheckpointCounts;
use std::fs;
use tempfile::TempDir;

/// Create a checkpoint ref pointing at HEAD by name using
/// `create_ghost_commit`. The exact tree content doesn't matter for the
/// counting tests — we just need refs under `refs/fspec-checkpoints/...`.
fn make_checkpoint(repo_path: &std::path::Path, work_unit_id: &str, name: &str) {
    // Write a small unique file so each checkpoint sees changes (the
    // `create_ghost_commit` helper short-circuits when nothing changed).
    let f = repo_path.join(format!("touch-{work_unit_id}-{name}.txt"));
    fs::write(&f, format!("{work_unit_id}/{name}")).expect("write touch file");
    ghost_commit::create_ghost_commit(repo_path, work_unit_id, name)
        .expect("create_ghost_commit");
}

/// Scenario: count_checkpoints returns zero in a directory that is not a git repo
#[test]
fn count_checkpoints_returns_zero_in_a_directory_that_is_not_a_git_repo() {
    // @step Given a temporary directory that has NOT been initialized as a git repository
    let tmp = TempDir::new().expect("tempdir");
    // @step When codelet_git::ghost_commit::count_checkpoints is called with that directory
    let counts = ghost_commit::count_checkpoints(tmp.path()).expect("count_checkpoints");
    // @step Then the call succeeds and returns CheckpointCounts { manual: 0, auto: 0 }
    assert_eq!(counts, CheckpointCounts { manual: 0, auto: 0 });
}

/// Scenario: count_checkpoints returns zero in a git repo with no checkpoint refs
#[test]
fn count_checkpoints_returns_zero_in_a_git_repo_with_no_checkpoint_refs() {
    // @step Given a temporary directory initialized as a git repository
    let tmp = common::setup_test_repo();
    // @step And no refs exist under refs/fspec-checkpoints/
    // (just-initialised repo from fixture has no fspec-checkpoint refs)
    // @step When codelet_git::ghost_commit::count_checkpoints is called against that repo
    let counts = ghost_commit::count_checkpoints(tmp.path()).expect("count_checkpoints");
    // @step Then the call returns CheckpointCounts { manual: 0, auto: 0 }
    assert_eq!(counts, CheckpointCounts { manual: 0, auto: 0 });
}

/// Scenario: count_checkpoints classifies one manual and one auto checkpoint for a single work unit
#[test]
fn count_checkpoints_classifies_one_manual_and_one_auto_for_a_single_work_unit() {
    // @step Given a temporary git repository
    let tmp = common::setup_test_repo();
    let repo_path = tmp.path();
    // @step And a ghost-checkpoint ref refs/fspec-checkpoints/AUTH-001/baseline exists (manual)
    make_checkpoint(repo_path, "AUTH-001", "baseline");
    // @step And a ghost-checkpoint ref refs/fspec-checkpoints/AUTH-001/AUTH-001-auto-testing exists (auto)
    make_checkpoint(repo_path, "AUTH-001", "AUTH-001-auto-testing");
    // @step When codelet_git::ghost_commit::count_checkpoints is called against that repo
    let counts = ghost_commit::count_checkpoints(repo_path).expect("count_checkpoints");
    // @step Then the result equals CheckpointCounts { manual: 1, auto: 1 }
    assert_eq!(counts, CheckpointCounts { manual: 1, auto: 1 });
}

/// Scenario: count_checkpoints aggregates counts across multiple work units
#[test]
fn count_checkpoints_aggregates_counts_across_multiple_work_units() {
    // @step Given a temporary git repository
    let tmp = common::setup_test_repo();
    let repo_path = tmp.path();
    // @step And a ref refs/fspec-checkpoints/AUTH-001/baseline exists (manual)
    make_checkpoint(repo_path, "AUTH-001", "baseline");
    // @step And a ref refs/fspec-checkpoints/AUTH-001/AUTH-001-auto-testing exists (auto)
    make_checkpoint(repo_path, "AUTH-001", "AUTH-001-auto-testing");
    // @step And a ref refs/fspec-checkpoints/BUG-002/BUG-002-auto-specifying exists (auto)
    make_checkpoint(repo_path, "BUG-002", "BUG-002-auto-specifying");
    // @step When codelet_git::ghost_commit::count_checkpoints is called against that repo
    let counts = ghost_commit::count_checkpoints(repo_path).expect("count_checkpoints");
    // @step Then the result equals CheckpointCounts { manual: 1, auto: 2 }
    assert_eq!(counts, CheckpointCounts { manual: 1, auto: 2 });
}

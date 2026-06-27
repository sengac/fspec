//! RPC-362 — Integration tests for the checkpoint transport helpers in
//! `codelet-rpc` and the new ghost-commit helpers they delegate to.
//!
//! Feature: spec/features/checkpoint-transport.feature
//!
//! Integration-first: every test builds a real temp git repo via the shared
//! `common::setup_test_repo` fixture and creates real ghost-commit checkpoints,
//! then exercises the `codelet_rpc::checkpoints` collection helpers end-to-end.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_rpc::checkpoints::{
    checkpoint_file_diff, collect_checkpoint_diff_files, collect_checkpoints, delete_all, delete_one,
    restore_all, restore_file,
};

use codelet_git::ghost_commit::create_ghost_commit;
use std::fs;
use std::path::Path;

/// Create a checkpoint by writing a unique file then snapshotting the worktree.
fn make_checkpoint(repo: &Path, work_unit_id: &str, name: &str) {
    let marker = repo.join(format!("touch-{work_unit_id}-{name}.txt"));
    fs::write(&marker, format!("{work_unit_id}/{name}")).expect("write marker");
    create_ghost_commit(repo, work_unit_id, name).expect("create_ghost_commit");
}

/// Write an index sidecar so collect_checkpoints can order by timestamp.
fn write_index(repo: &Path, work_unit_id: &str, entries: &[(&str, &str)]) {
    let dir = repo.join(".git").join("fspec-checkpoints-index");
    fs::create_dir_all(&dir).expect("mkdir index dir");
    let checkpoints: Vec<serde_json::Value> = entries
        .iter()
        .map(|(name, ts)| serde_json::json!({ "name": name, "timestamp": ts }))
        .collect();
    let body = serde_json::json!({ "checkpoints": checkpoints });
    fs::write(
        dir.join(format!("{work_unit_id}.json")),
        serde_json::to_string_pretty(&body).unwrap(),
    )
    .expect("write index");
}

/// Scenario: list_checkpoints returns checkpoints most-recent-first with automatic flags
#[test]
fn list_checkpoints_returns_most_recent_first_with_automatic_flags() {
    // @step Given a git repository with three checkpoints created in order baseline, AUTH-001-auto-a, AUTH-001-auto-b
    let tmp = common::setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "baseline");
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-a");
    make_checkpoint(repo, "AUTH-001", "AUTH-001-auto-b");
    write_index(
        repo,
        "AUTH-001",
        &[
            ("baseline", "2026-06-01T10:00:00.000Z"),
            ("AUTH-001-auto-a", "2026-06-02T10:00:00.000Z"),
            ("AUTH-001-auto-b", "2026-06-03T10:00:00.000Z"),
        ],
    );

    // @step When I call the list_checkpoints helper against that repository
    let list = collect_checkpoints(repo).expect("collect_checkpoints");

    // @step Then it returns three CheckpointInfo entries most-recent-first
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].name, "AUTH-001-auto-b");
    assert_eq!(list[1].name, "AUTH-001-auto-a");
    assert_eq!(list[2].name, "baseline");

    // @step And the is_automatic flags are true, true, false in returned order
    assert_eq!(
        list.iter().map(|c| c.is_automatic).collect::<Vec<_>>(),
        vec![true, true, false]
    );
}

/// Scenario: list_checkpoints caps the result at 200 entries
#[test]
fn list_checkpoints_caps_the_result_at_200_entries() {
    // @step Given a git repository with 250 checkpoints
    let tmp = common::setup_test_repo();
    let repo = tmp.path();
    let mut entries: Vec<(String, String)> = Vec::new();
    for i in 0..250 {
        let name = format!("cp-{i:03}");
        make_checkpoint(repo, "AUTH-001", &name);
        // Timestamps strictly increase so ordering is deterministic.
        entries.push((name, format!("2026-06-01T00:{:02}:{:02}.000Z", i / 60, i % 60)));
    }
    let refs: Vec<(&str, &str)> = entries
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    write_index(repo, "AUTH-001", &refs);

    // @step When I call the list_checkpoints helper against that repository
    let list = collect_checkpoints(repo).expect("collect_checkpoints");

    // @step Then it returns exactly 200 CheckpointInfo entries
    assert_eq!(list.len(), 200);
}

/// Scenario: checkpoint_diff_files returns one ChangedFile per changed file
#[test]
fn checkpoint_diff_files_returns_one_changed_file_per_changed_file() {
    // @step Given a checkpoint whose working tree differs in a.txt and b.txt
    let tmp = common::setup_test_repo();
    let repo = tmp.path();
    fs::write(repo.join("a.txt"), "a-v1\n").expect("write a");
    fs::write(repo.join("b.txt"), "b-v1\n").expect("write b");
    create_ghost_commit(repo, "AUTH-001", "cp").expect("create checkpoint");
    fs::write(repo.join("a.txt"), "a-v2\n").expect("modify a");
    fs::write(repo.join("b.txt"), "b-v2\n").expect("modify b");

    // @step When I call the checkpoint_diff_files helper for that checkpoint
    let files = collect_checkpoint_diff_files(repo, "AUTH-001", "cp").expect("diff files");

    // @step Then it returns two ChangedFile entries for a.txt and b.txt
    let mut paths: Vec<String> = files.into_iter().map(|f| f.path).collect();
    paths.sort();
    assert_eq!(paths, vec!["a.txt".to_string(), "b.txt".to_string()]);
}

/// Scenario: checkpoint_file_diff returns the unified diff for a changed file
#[test]
fn checkpoint_file_diff_returns_the_unified_diff_for_a_changed_file() {
    // @step Given a checkpoint whose working tree differs in a.txt
    let tmp = common::setup_test_repo();
    let repo = tmp.path();
    fs::write(repo.join("a.txt"), "original\n").expect("write a");
    // Commit so HEAD has the original; checkpoint captures original too.
    create_ghost_commit(repo, "AUTH-001", "cp").expect("create checkpoint");
    fs::write(repo.join("a.txt"), "changed\n").expect("modify a");

    // @step When I call the checkpoint_file_diff helper for a.txt
    let diff = checkpoint_file_diff(repo, "AUTH-001", "cp", "a.txt").expect("file diff");

    // @step Then it returns Some unified diff text for a.txt
    let diff = diff.expect("some diff");
    assert!(diff.contains("a.txt"), "diff should mention the file: {diff}");
}

/// Scenario: restore_checkpoint_all restores the working tree
#[test]
fn restore_checkpoint_all_restores_the_working_tree() {
    // @step Given a checkpoint and a working tree modified after it
    let tmp = common::setup_test_repo();
    let repo = tmp.path();
    fs::write(repo.join("a.txt"), "snapshot\n").expect("write a");
    create_ghost_commit(repo, "AUTH-001", "cp").expect("create checkpoint");
    fs::write(repo.join("a.txt"), "drifted\n").expect("modify a");

    // @step When I call the restore_checkpoint_all helper for that checkpoint
    restore_all(repo, "AUTH-001", "cp").expect("restore_all returns Ok");

    // @step Then the call returns Ok
    assert_eq!(
        fs::read_to_string(repo.join("a.txt")).expect("read a"),
        "snapshot\n"
    );

    // @step And a subsequent checkpoint_diff_files reports no changed files
    let files = collect_checkpoint_diff_files(repo, "AUTH-001", "cp").expect("diff files");
    assert!(files.is_empty(), "expected no diff after restore: {files:?}");
}

/// Scenario: delete_checkpoint removes one checkpoint
#[test]
fn delete_checkpoint_removes_one_checkpoint() {
    // @step Given a git repository with two checkpoints
    let tmp = common::setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "first");
    make_checkpoint(repo, "AUTH-001", "second");
    let before = collect_checkpoints(repo).expect("collect before");
    assert_eq!(before.len(), 2);

    // @step When I call the delete_checkpoint helper for one checkpoint
    delete_one(repo, "AUTH-001", "first").expect("delete_one");

    // @step Then a subsequent list_checkpoints returns one fewer entry
    let after = collect_checkpoints(repo).expect("collect after");
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].name, "second");
}

/// Scenario: restore_checkpoint_file restores a single file
#[test]
fn restore_checkpoint_file_restores_a_single_file() {
    // @step Given a checkpoint and a single file modified after it
    let tmp = common::setup_test_repo();
    let repo = tmp.path();
    fs::write(repo.join("a.txt"), "snapshot\n").expect("write a");
    create_ghost_commit(repo, "AUTH-001", "cp").expect("create checkpoint");
    fs::write(repo.join("a.txt"), "drifted\n").expect("modify a");

    // @step When I call the restore_checkpoint_file helper for a.txt
    restore_file(repo, "AUTH-001", "cp", "a.txt").expect("restore_file returns Ok");

    // @step Then the file content matches the checkpoint snapshot
    assert_eq!(
        fs::read_to_string(repo.join("a.txt")).expect("read a"),
        "snapshot\n"
    );
}

/// Scenario: delete_all_checkpoints removes every checkpoint
#[test]
fn delete_all_checkpoints_removes_every_checkpoint() {
    // @step Given a git repository with three checkpoints
    let tmp = common::setup_test_repo();
    let repo = tmp.path();
    make_checkpoint(repo, "AUTH-001", "first");
    make_checkpoint(repo, "AUTH-001", "second");
    make_checkpoint(repo, "AUTH-002", "third");
    let before = collect_checkpoints(repo).expect("collect before");
    assert_eq!(before.len(), 3);

    // @step When I call the delete_all_checkpoints helper
    delete_all(repo).expect("delete_all");

    // @step Then a subsequent list_checkpoints returns no entries
    let after = collect_checkpoints(repo).expect("collect after");
    assert!(after.is_empty(), "expected no checkpoints: {after:?}");
}

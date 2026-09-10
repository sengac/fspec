//! WT-010 — restore-checkpoint command surfaces a lower-layer conflict as a
//! conflict result, not a not-found sentinel.
//!
//! Feature: spec/features/restore-ghost-commit-ignores-the-force-parameter-no-conflict-detection-before-overwriting.feature
//!
//! Scenario 4 of the feature: when the working tree is git-clean (the change
//! was committed after the checkpoint) the fspec-core dirty-tree pre-check is
//! bypassed and the divergence is only discovered by
//! `codelet_git::ghost_commit::restore_ghost_commit` itself. Until WT-010's
//! lower-layer fix, that `Err(ConflictError)` falls into the `Err(_)` arm of
//! `restore_util` and degrades to the "Checkpoint ... not found" sentinel —
//! this test pins the mapping.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "restore-checkpoint".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn init_git_repo(dir: &Path) {
    for args in [
        vec!["init", "--quiet"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test User"],
    ] {
        Command::new("git")
            .args(&args)
            .current_dir(dir)
            .status()
            .expect("git setup");
    }
    fs::write(dir.join("README.md"), "# test\n").expect("seed README");
    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(dir)
        .status()
        .expect("git add");
    Command::new("git")
        .args(["commit", "--quiet", "-m", "initial"])
        .current_dir(dir)
        .status()
        .expect("git commit");
}

fn git_commit_all(dir: &Path, msg: &str) {
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .status()
        .expect("git add -A");
    Command::new("git")
        .args(["commit", "--quiet", "-m", msg])
        .current_dir(dir)
        .status()
        .expect("git commit");
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

// =============================================================================
// Scenario: The restore-checkpoint command surfaces a lower-layer conflict
// as a conflict result, not a not-found sentinel
// =============================================================================

#[test]
fn restore_checkpoint_surfaces_a_lower_layer_conflict_as_a_conflict_result() {
    // @step Given a git repository where test.txt was committed as 'v1' and a ghost commit checkpoint was taken, then test.txt was changed to 'v2' and committed so the working tree reports clean
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    fs::write(tmp.path().join("test.txt"), "v1\n").expect("write v1");
    git_commit_all(tmp.path(), "commit v1");
    codelet_git::ghost_commit::create_ghost_commit(tmp.path(), "WT010", "snap")
        .expect("create checkpoint");
    fs::write(tmp.path().join("test.txt"), "v2\n").expect("write v2");
    git_commit_all(tmp.path(), "commit v2"); // working tree now clean

    // @step When the dispatcher receives restore-checkpoint for that checkpoint with force false and no userChoice and the dirty check is reported clean
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "WT010", "checkpointName": "snap" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the result reports success false, conflictsDetected true, conflictedFiles containing test.txt, and a CHECKPOINT RESTORATION CONFLICT DETECTED system reminder
    assert_eq!(
        data["success"].as_bool(),
        Some(false),
        "a divergent non-forced restore must not report success"
    );
    assert_eq!(
        data["conflictsDetected"].as_bool(),
        Some(true),
        "WT-010: a lower-layer ConflictError must surface as conflictsDetected=true, \
         not as the not-found sentinel"
    );
    let conflicted = data["conflictedFiles"]
        .as_array()
        .expect("conflictedFiles array");
    assert!(
        conflicted.iter().any(|f| f.as_str() == Some("test.txt")),
        "conflictedFiles must list test.txt; got {conflicted:?}"
    );
    let reminder = data["systemReminder"]
        .as_str()
        .expect("systemReminder string");
    assert!(
        reminder.contains("CHECKPOINT RESTORATION CONFLICT DETECTED"),
        "systemReminder must carry the conflict header, not a not-found sentinel; got:\n{reminder}"
    );
    assert!(
        !reminder.contains("not found for work unit"),
        "the not-found sentinel must not be produced for a conflict; got:\n{reminder}"
    );
}

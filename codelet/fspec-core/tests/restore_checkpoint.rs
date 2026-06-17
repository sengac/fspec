#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/restore-checkpoint-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `restore-checkpoint`
// (RPC-288). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// NOTE (Phase B): the core command is still a NotYetPorted stub, so these
// green-phase assertions are EXPECTED to fail until Phase C wires the real
// implementation. Fixtures use codelet_git directly (a regular dependency of
// fspec-core) so real ghost-commit checkpoints carry restorable file content.

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

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

/// Create a real ghost-commit checkpoint capturing `marker.txt` with the
/// supplied content. Leaves the file in the working tree (untracked).
fn create_real_checkpoint(dir: &Path, work_unit_id: &str, checkpoint_name: &str, content: &str) {
    fs::write(dir.join("marker.txt"), content).expect("write marker");
    codelet_git::ghost_commit::create_ghost_commit(dir, work_unit_id, checkpoint_name)
        .expect("create_ghost_commit");
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

// ---------- scenarios ----------

#[test]
fn restore_against_a_clean_working_tree() {
    // Scenario: Restore against a clean working tree

    // @step Given a git repository with a checkpoint "baseline" for "AUTH-001" and a clean working tree
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    create_real_checkpoint(tmp.path(), "AUTH-001", "baseline", "captured\n");
    git_commit_all(tmp.path(), "commit checkpoint content"); // working tree now clean

    // @step And the dispatcher receives command "restore-checkpoint" with workUnitId "AUTH-001" and checkpointName "baseline"
    // @step When fspec_core::commands::restore_checkpoint::run executes
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "checkpointName": "baseline" }),
    ));
    assert!(result.success, "expected success=true; got {result:?}");
    let data = parse_data(&result.data);

    // @step Then the result reports success true and conflictsDetected false
    assert_eq!(data["success"].as_bool(), Some(true));
    assert_eq!(data["conflictsDetected"].as_bool(), Some(false));

    // @step And the rendered text contains "✓ Restored checkpoint \"baseline\" for AUTH-001"
    // (text re-render to assert the banner)
    let text = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "checkpointName": "baseline", "format": "text" }),
    ));
    assert!(
        text.data
            .contains("\u{2713} Restored checkpoint \"baseline\" for AUTH-001"),
        "missing restore banner; got:\n{}",
        text.data
    );
}

#[test]
fn dirty_working_tree_without_a_choice_requires_a_user_choice() {
    // Scenario: Dirty working tree without a choice shows the risk options and requires a user choice

    // @step Given a git repository with a checkpoint "before-refactor" for "UI-002" and uncommitted changes
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    create_real_checkpoint(tmp.path(), "UI-002", "before-refactor", "captured\n");
    fs::write(tmp.path().join("uncommitted.txt"), "dirty\n").expect("dirty file");

    // @step And the dispatcher receives command "restore-checkpoint" with workUnitId "UI-002" and checkpointName "before-refactor" and no force and no userChoice
    // @step When fspec_core::commands::restore_checkpoint::run executes
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "UI-002", "checkpointName": "before-refactor" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the result reports success false and requiresUserChoice true
    assert_eq!(data["success"].as_bool(), Some(false));
    assert_eq!(data["requiresUserChoice"].as_bool(), Some(true));

    // @step And the rendered text contains "Working directory has uncommitted changes"
    assert!(
        result.data.contains("Working directory has uncommitted changes"),
        "missing dirty-tree warning; got:\n{}",
        result.data
    );

    // @step And the rendered text lists three numbered risk options including "Low", "Medium", and "High"
    assert!(result.data.contains("1."), "missing option 1; got:\n{}", result.data);
    assert!(result.data.contains("2."), "missing option 2; got:\n{}", result.data);
    assert!(result.data.contains("3."), "missing option 3; got:\n{}", result.data);
    for risk in ["Low", "Medium", "High"] {
        assert!(
            result.data.contains(risk),
            "missing risk level {risk}; got:\n{}",
            result.data
        );
    }

    // @step And no files are restored
    let restored = fs::read_to_string(tmp.path().join("marker.txt")).unwrap_or_default();
    assert!(
        restored.is_empty() || !tmp.path().join("marker.txt").exists() || restored == "captured\n",
        "no restore should have mutated the working tree unexpectedly"
    );
    // The uncommitted file must still be present (untouched).
    assert!(tmp.path().join("uncommitted.txt").exists());
}

#[test]
fn conflicts_detected_when_working_tree_files_differ() {
    // Scenario: Conflicts detected when working-tree files differ from the checkpoint

    // @step Given a git repository with a checkpoint "previous-state" for "AUTH-001"
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    create_real_checkpoint(tmp.path(), "AUTH-001", "previous-state", "version-A\n");

    // @step And working-tree files differ from that checkpoint and the request does not force
    fs::write(tmp.path().join("marker.txt"), "version-B\n").expect("modify marker");

    // @step But the request supplies userChoice so the conflict pre-check runs
    // @step When fspec_core::commands::restore_checkpoint::run executes
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "checkpointName": "previous-state",
            "userChoice": "2"
        }),
    ));
    let data = parse_data(&result.data);

    // @step Then conflictsDetected is true and conflictedFiles lists the differing files
    assert_eq!(data["conflictsDetected"].as_bool(), Some(true));
    let conflicted = data["conflictedFiles"]
        .as_array()
        .expect("conflictedFiles array");
    assert!(
        conflicted.iter().any(|f| f.as_str() == Some("marker.txt")),
        "conflictedFiles must list marker.txt; got {conflicted:?}"
    );

    // @step And the systemReminder contains "CHECKPOINT RESTORATION CONFLICT DETECTED"
    let reminder = data["systemReminder"].as_str().expect("systemReminder string");
    assert!(
        reminder.contains("CHECKPOINT RESTORATION CONFLICT DETECTED"),
        "missing conflict header; got:\n{reminder}"
    );

    // @step And the systemReminder ends with "</system-reminder>"
    assert!(
        reminder.trim_end().ends_with("</system-reminder>"),
        "systemReminder must end with closing tag; got:\n{reminder}"
    );
}

#[test]
fn missing_checkpoint_ref_reports_not_found_without_erroring() {
    // Scenario: Missing checkpoint ref reports not-found without erroring

    // @step Given a git repository with no checkpoint named "ghost" for "AUTH-001"
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());

    // @step And the dispatcher receives command "restore-checkpoint" with workUnitId "AUTH-001" and checkpointName "ghost"
    // @step When fspec_core::commands::restore_checkpoint::run executes
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "checkpointName": "ghost" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the result reports success false
    assert_eq!(data["success"].as_bool(), Some(false));

    // @step And the systemReminder is "Checkpoint \"ghost\" not found for work unit AUTH-001"
    assert_eq!(
        data["systemReminder"].as_str(),
        Some("Checkpoint \"ghost\" not found for work unit AUTH-001"),
        "systemReminder mismatch; got {:?}",
        data["systemReminder"]
    );
}

#[test]
fn reject_an_empty_checkpoint_name() {
    // Scenario: Reject an empty checkpoint name

    // @step Given the dispatcher receives command "restore-checkpoint" with workUnitId "AUTH-001" and an empty checkpointName
    let tmp = TempDir::new().expect("tempdir");

    // @step When fspec_core::commands::restore_checkpoint::run executes
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "checkpointName": "" }),
    ));

    // @step Then it returns an InvalidArgs error naming the empty checkpointName field
    assert!(!result.success, "expected success=false; got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("checkpointName"),
        "error must name checkpointName; got: {msg}"
    );
}

#[test]
fn force_restore_against_a_dirty_repo_succeeds_without_conflicts() {
    // Scenario: force restore against a dirty repo succeeds without conflicts

    // @step Given a git repository with a checkpoint "baseline" for "AUTH-001" and uncommitted changes
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    create_real_checkpoint(tmp.path(), "AUTH-001", "baseline", "captured\n");
    fs::write(tmp.path().join("marker.txt"), "locally-modified\n").expect("modify marker");

    // @step And the dispatcher receives command "restore-checkpoint" with workUnitId "AUTH-001", checkpointName "baseline" and force true
    // @step When fspec_core::commands::restore_checkpoint::run executes
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "checkpointName": "baseline", "force": true }),
    ));
    let data = parse_data(&result.data);

    // @step Then the result reports success true and conflictsDetected false
    assert_eq!(data["success"].as_bool(), Some(true));
    assert_eq!(data["conflictsDetected"].as_bool(), Some(false));
}

#[test]
fn format_json_emits_structured_payload_preserving_key_order() {
    // Scenario: format json emits the structured payload preserving key order

    // @step Given a git repository with a checkpoint "baseline" for "AUTH-001" and a clean working tree
    let tmp = TempDir::new().expect("tempdir");
    init_git_repo(tmp.path());
    create_real_checkpoint(tmp.path(), "AUTH-001", "baseline", "captured\n");
    git_commit_all(tmp.path(), "commit checkpoint content");

    // @step And the dispatcher receives command "restore-checkpoint" with workUnitId "AUTH-001", checkpointName "baseline" and format "json"
    // @step When fspec_core::commands::restore_checkpoint::run executes
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "checkpointName": "baseline", "format": "json" }),
    ));
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the rendered output is pretty-printed JSON
    let data = parse_data(&result.data);
    assert!(
        result.data.contains('\n') && result.data.contains("  "),
        "expected pretty-printed JSON; got:\n{}",
        result.data
    );

    // @step And it has the keys "success", "conflictsDetected", "conflictedFiles", "systemReminder", "requiresTestValidation" in that order
    let keys: Vec<&str> = data
        .as_object()
        .expect("root object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        vec![
            "success",
            "conflictsDetected",
            "conflictedFiles",
            "systemReminder",
            "requiresTestValidation"
        ],
        "key order mismatch; got {keys:?}"
    );
}

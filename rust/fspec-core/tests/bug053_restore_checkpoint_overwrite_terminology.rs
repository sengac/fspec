//! BUG-053 — restore-checkpoint names its third option an overwrite, not a merge.
//!
//! Feature: spec/features/checkpoint-restore-shows-confusing-merge-terminology-when-it-actually-overwrites-files.feature
//!
//! With a dirty working tree and no choice, `restore-checkpoint` returns the
//! three risk options; the third overwrites the working tree with the
//! checkpoint, and no option speaks of merging or resolving conflicts.

use std::path::Path;
use std::process::Command;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const OVERWRITE: &str = "Overwrite files (discard changes)";

fn git(dir: &Path, args: &[&str]) -> TestResult {
    let status = Command::new("git").args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed: {status}").into());
    }
    Ok(())
}

/// A git repository with checkpoint `baseline` of AUTH-001 capturing
/// marker.txt = "captured", then marker.txt changed to "dirty" (uncommitted).
fn dirty_repo_with_baseline() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();
    git(dir, &["init", "--quiet"])?;
    git(dir, &["config", "user.email", "test@example.com"])?;
    git(dir, &["config", "user.name", "Test User"])?;
    git(dir, &["config", "commit.gpgsign", "false"])?;
    std::fs::write(dir.join("README.md"), "# test\n")?;
    git(dir, &["add", "README.md"])?;
    git(dir, &["commit", "--quiet", "-m", "initial"])?;
    std::fs::write(dir.join("marker.txt"), "captured\n")?;
    codelet_git::ghost_commit::create_ghost_commit(dir, "AUTH-001", "baseline")?;
    std::fs::write(dir.join("marker.txt"), "dirty\n")?;
    Ok(tmp)
}

fn restore(dir: &Path, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: "restore-checkpoint".to_string(),
        args_json: args.to_string(),
        project_root: dir.to_path_buf(),
    })
}

/// The `options` array of the requires-user-choice payload.
fn prompt(dir: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let result = restore(
        dir,
        json!({ "workUnitId": "AUTH-001", "checkpointName": "baseline" }),
    );
    let data: Value = serde_json::from_str(&result.data)?;
    if data["requiresUserChoice"] != json!(true) {
        return Err(format!("no user choice requested: {result:?}").into());
    }
    Ok(data)
}

#[test]
fn restore_with_dirty_working_directory_shows_accurate_overwrite_option() -> TestResult {
    // @step Given I have a work unit AUTH-001 with a checkpoint named 'baseline'
    // @step And I have uncommitted changes in my working directory
    let tmp = dirty_repo_with_baseline()?;

    // @step When I run 'fspec restore-checkpoint AUTH-001 baseline'
    let data = prompt(tmp.path())?;
    let option = &data["options"][2];

    // @step Then option 3 should be labeled 'Overwrite files (discard changes)'
    assert_eq!(option["name"], OVERWRITE, "{data}");
    let message = data["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("  3. Overwrite files (discard changes) [High risk]\n"),
        "{message}"
    );

    // @step And option 3 should have risk level 'High'
    assert_eq!(option["riskLevel"], "High", "{data}");

    // @step And option 3 description should warn 'Overwrites working directory with checkpoint. Current changes will be LOST FOREVER unless committed or stashed.'
    assert_eq!(
        option["description"],
        "Overwrites working directory with checkpoint. Current changes will be LOST FOREVER unless committed or stashed.",
        "{data}"
    );

    // Nothing was restored while the choice is pending.
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("marker.txt"))?,
        "dirty\n"
    );
    Ok(())
}

#[test]
fn option_text_removes_all_misleading_merge_terminology() -> TestResult {
    // @step Given I have a work unit with uncommitted changes
    let tmp = dirty_repo_with_baseline()?;

    // @step When I view the restore-checkpoint prompt options
    let data = prompt(tmp.path())?;
    let options = data["options"].as_array().cloned().unwrap_or_default();
    assert_eq!(options.len(), 3, "{data}");
    let message = data["message"].as_str().unwrap_or_default().to_lowercase();

    // @step Then no option should contain the word 'merge'
    for option in &options {
        let name = option["name"].as_str().unwrap_or_default().to_lowercase();
        let description = option["description"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase();
        assert!(!name.contains("merge"), "{option}");
        assert!(!description.contains("merge"), "{option}");
    }
    assert!(!message.contains("merge"), "{message}");

    // @step And no option description should mention 'conflicts' or 'manual resolution'
    for option in &options {
        let description = option["description"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase();
        assert!(!description.contains("conflict"), "{option}");
        assert!(!description.contains("manual resolution"), "{option}");
    }
    assert!(!message.contains("conflict"), "{message}");

    // @step And the terminology should accurately reflect pure file overwrite behavior
    let chosen = restore(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "checkpointName": "baseline", "userChoice": OVERWRITE }),
    );
    let result: Value = serde_json::from_str(&chosen.data)?;
    assert_eq!(result["success"], true, "{chosen:?}");
    assert_eq!(result["conflictsDetected"], false, "{chosen:?}");
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("marker.txt"))?,
        "captured\n",
        "the overwrite choice replaces the file with the checkpoint's content"
    );
    Ok(())
}

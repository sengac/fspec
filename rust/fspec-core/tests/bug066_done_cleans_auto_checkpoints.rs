//! BUG-066 — moving a work unit to done deletes its automatic checkpoints
//! and keeps its manual ones.
//!
//! Feature: spec/features/auto-checkpoints-not-cleaned-up-when-work-unit-moves-to-done.feature
//!
//! `update-work-unit-status <id> done` deletes every
//! `refs/fspec-checkpoints/<id>/<name>` ref whose name holds `-auto-`
//! (`cleanup_done_checkpoints`). Checkpoints are checked twice: through
//! `list-checkpoints` and through the git refs themselves. The checkpoint
//! index file is out of scope (product owner, 2026-11-06: refs only).

use std::path::Path;
use std::process::Command;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn git(dir: &Path, args: &[&str]) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let output = Command::new("git").args(args).current_dir(dir).output()?;
    if !output.status.success() {
        return Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(output)
}

/// A git repository whose only work unit `id` (of `work_type`) is
/// validating, with every file committed.
fn repo(id: &str, work_type: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();
    git(dir, &["init", "--quiet"])?;
    git(dir, &["config", "user.email", "test@example.com"])?;
    git(dir, &["config", "user.name", "Test User"])?;
    git(dir, &["config", "commit.gpgsign", "false"])?;
    let mut states = serde_json::Map::new();
    for state in [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let ids: Vec<&str> = if state == "validating" {
            vec![id]
        } else {
            vec![]
        };
        states.insert(state.to_string(), json!(ids));
    }
    std::fs::create_dir_all(dir.join("spec"))?;
    std::fs::write(
        dir.join("spec/work-units.json"),
        serde_json::to_string_pretty(&json!({
            "version": "0.7.1",
            "meta": {"version": "1.0.0", "lastUpdated": "2026-11-01T00:00:00.000Z"},
            "workUnits": {id: {
                "id": id, "title": "Cleanup", "type": work_type, "status": "validating",
                "createdAt": "2026-11-01T00:00:00.000Z", "updatedAt": "2026-11-01T00:00:00.000Z"
            }},
            "states": states,
        }))?,
    )?;
    git(dir, &["add", "-A"])?;
    git(dir, &["commit", "--quiet", "-m", "initial"])?;
    Ok(tmp)
}

/// Checkpoint `name` of `id` as a ref at HEAD.
fn checkpoint(dir: &Path, id: &str, name: &str) -> TestResult {
    git(
        dir,
        &[
            "update-ref",
            &format!("refs/fspec-checkpoints/{id}/{name}"),
            "HEAD",
        ],
    )?;
    Ok(())
}

fn ref_exists(dir: &Path, id: &str, name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let out = git(
        dir,
        &[
            "for-each-ref",
            "--format=%(refname)",
            &format!("refs/fspec-checkpoints/{id}/"),
        ],
    )?;
    let wanted = format!("refs/fspec-checkpoints/{id}/{name}");
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .any(|line| line.trim() == wanted))
}

/// The checkpoint names `list-checkpoints` reports for `id`, sorted.
fn listed(dir: &Path, id: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let result = dispatch(
        dir,
        "list-checkpoints",
        json!({"workUnitId": id, "format": "json"}),
    );
    if !result.success {
        return Err(format!("list-checkpoints failed: {result:?}").into());
    }
    let data: Value = serde_json::from_str(&result.data)?;
    let mut names: Vec<String> = data["checkpoints"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|checkpoint| checkpoint["name"].as_str())
        .map(str::to_string)
        .collect();
    names.sort();
    Ok(names)
}

fn dispatch(dir: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: dir.to_path_buf(),
    })
}

fn move_to_done(dir: &Path, id: &str) -> DispatchResult {
    dispatch(
        dir,
        "update-work-unit-status",
        json!({"workUnitId": id, "status": "done"}),
    )
}

fn succeeded(result: &DispatchResult) -> TestResult {
    if !result.success {
        return Err(format!("the move to done failed: {result:?}").into());
    }
    Ok(())
}

#[test]
fn work_unit_with_mixed_auto_and_manual_checkpoints() -> TestResult {
    // @step Given a work unit "AUTH-001" exists with status "validating"
    let tmp = repo("AUTH-001", "story")?;
    let dir = tmp.path();

    // @step And "AUTH-001" has automatic checkpoint "AUTH-001-auto-specifying"
    checkpoint(dir, "AUTH-001", "AUTH-001-auto-specifying")?;

    // @step And "AUTH-001" has automatic checkpoint "AUTH-001-auto-testing"
    checkpoint(dir, "AUTH-001", "AUTH-001-auto-testing")?;

    // @step And "AUTH-001" has manual checkpoint "before-major-refactor"
    checkpoint(dir, "AUTH-001", "before-major-refactor")?;
    assert_eq!(listed(dir, "AUTH-001")?.len(), 3);

    // @step When I run "fspec update-work-unit-status AUTH-001 done"
    let result = move_to_done(dir, "AUTH-001");

    // @step Then the command should succeed
    succeeded(&result)?;
    let after = listed(dir, "AUTH-001")?;

    // @step And checkpoint "AUTH-001-auto-specifying" should be deleted
    assert!(
        !after.contains(&"AUTH-001-auto-specifying".to_string()),
        "{after:?}"
    );

    // @step And checkpoint "AUTH-001-auto-testing" should be deleted
    assert!(
        !after.contains(&"AUTH-001-auto-testing".to_string()),
        "{after:?}"
    );

    // @step And checkpoint "before-major-refactor" should exist
    assert_eq!(after, vec!["before-major-refactor".to_string()]);

    // @step And the git ref "refs/fspec-checkpoints/AUTH-001/AUTH-001-auto-specifying" should not exist
    assert!(!ref_exists(dir, "AUTH-001", "AUTH-001-auto-specifying")?);

    // @step And the git ref "refs/fspec-checkpoints/AUTH-001/AUTH-001-auto-testing" should not exist
    assert!(!ref_exists(dir, "AUTH-001", "AUTH-001-auto-testing")?);

    // @step And the git ref "refs/fspec-checkpoints/AUTH-001/before-major-refactor" should exist
    assert!(ref_exists(dir, "AUTH-001", "before-major-refactor")?);
    Ok(())
}

#[test]
fn work_unit_with_only_manual_checkpoints() -> TestResult {
    // @step Given a work unit "BUG-027" exists with status "validating"
    let tmp = repo("BUG-027", "bug")?;
    let dir = tmp.path();

    // @step And "BUG-027" has manual checkpoint "before-fix"
    checkpoint(dir, "BUG-027", "before-fix")?;

    // @step And "BUG-027" has manual checkpoint "after-tests"
    checkpoint(dir, "BUG-027", "after-tests")?;
    let before = listed(dir, "BUG-027")?;

    // @step When I run "fspec update-work-unit-status BUG-027 done"
    let result = move_to_done(dir, "BUG-027");

    // @step Then the command should succeed
    succeeded(&result)?;
    let after = listed(dir, "BUG-027")?;

    // @step And checkpoint "before-fix" should exist
    assert!(after.contains(&"before-fix".to_string()), "{after:?}");

    // @step And checkpoint "after-tests" should exist
    assert!(after.contains(&"after-tests".to_string()), "{after:?}");

    // @step And the git ref "refs/fspec-checkpoints/BUG-027/before-fix" should exist
    assert!(ref_exists(dir, "BUG-027", "before-fix")?);

    // @step And the git ref "refs/fspec-checkpoints/BUG-027/after-tests" should exist
    assert!(ref_exists(dir, "BUG-027", "after-tests")?);

    // @step And no checkpoints should be deleted
    assert_eq!(after, before);
    assert_eq!(
        after,
        vec!["after-tests".to_string(), "before-fix".to_string()]
    );
    Ok(())
}

#[test]
fn work_unit_with_only_automatic_checkpoints() -> TestResult {
    // @step Given a work unit "FEAT-010" exists with status "validating"
    let tmp = repo("FEAT-010", "story")?;
    let dir = tmp.path();

    // @step And "FEAT-010" has automatic checkpoint "FEAT-010-auto-backlog"
    checkpoint(dir, "FEAT-010", "FEAT-010-auto-backlog")?;

    // @step And "FEAT-010" has automatic checkpoint "FEAT-010-auto-specifying"
    checkpoint(dir, "FEAT-010", "FEAT-010-auto-specifying")?;
    assert_eq!(listed(dir, "FEAT-010")?.len(), 2);

    // @step When I run "fspec update-work-unit-status FEAT-010 done"
    let result = move_to_done(dir, "FEAT-010");

    // @step Then the command should succeed
    succeeded(&result)?;
    let after = listed(dir, "FEAT-010")?;

    // @step And checkpoint "FEAT-010-auto-backlog" should be deleted
    assert!(
        !after.contains(&"FEAT-010-auto-backlog".to_string()),
        "{after:?}"
    );

    // @step And checkpoint "FEAT-010-auto-specifying" should be deleted
    assert!(
        !after.contains(&"FEAT-010-auto-specifying".to_string()),
        "{after:?}"
    );

    // @step And the git ref "refs/fspec-checkpoints/FEAT-010/FEAT-010-auto-backlog" should not exist
    assert!(!ref_exists(dir, "FEAT-010", "FEAT-010-auto-backlog")?);

    // @step And the git ref "refs/fspec-checkpoints/FEAT-010/FEAT-010-auto-specifying" should not exist
    assert!(!ref_exists(dir, "FEAT-010", "FEAT-010-auto-specifying")?);

    // @step And all checkpoints for "FEAT-010" should be deleted
    assert!(after.is_empty(), "{after:?}");
    Ok(())
}

#[test]
fn work_unit_with_no_checkpoints() -> TestResult {
    // @step Given a work unit "TASK-001" exists with status "validating"
    let tmp = repo("TASK-001", "task")?;
    let dir = tmp.path();

    // @step And "TASK-001" has no checkpoints
    assert!(listed(dir, "TASK-001")?.is_empty());

    // @step When I run "fspec update-work-unit-status TASK-001 done"
    let result = move_to_done(dir, "TASK-001");

    // @step Then the command should succeed
    succeeded(&result)?;

    // @step And no errors should occur
    assert_eq!(result.error, None, "{result:?}");

    // @step And no checkpoints should be deleted
    assert!(listed(dir, "TASK-001")?.is_empty());
    Ok(())
}

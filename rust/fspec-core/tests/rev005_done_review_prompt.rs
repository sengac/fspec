//! REV-005 — moving a story or a bug to done suggests `fspec review`; a
//! task gets no review suggestion.
//!
//! Feature: spec/features/conversational-review-prompt-before-done.feature
//!
//! `update-work-unit-status <id> done` adds the QUALITY CHECK OPPORTUNITY
//! reminder for stories and bugs only (`done_review_reminder`), inside the
//! one consolidated `<system-reminder>` block. The units have no feature
//! files, so the done audit has nothing to check.

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const QUESTION: &str = "Would you like me to run fspec review";

/// A board whose only work unit `id` (of `work_type`) is validating.
fn board(dir: &Path, id: &str, work_type: &str) -> TestResult {
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
                "id": id, "title": "Finish", "type": work_type, "status": "validating",
                "createdAt": "2026-11-01T00:00:00.000Z", "updatedAt": "2026-11-01T00:00:00.000Z"
            }},
            "states": states,
        }))?,
    )?;
    Ok(())
}

fn move_to_done(dir: &Path, id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let result: DispatchResult = dispatch_command(DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: json!({"workUnitId": id, "status": "done"}).to_string(),
        project_root: dir.to_path_buf(),
    });
    if !result.success {
        return Err(format!("the move to done failed: {result:?}").into());
    }
    Ok(result.data)
}

/// The status of `id` saved in the board, and whether the done column
/// holds it.
fn saved_status(dir: &Path, id: &str) -> Result<(String, bool), Box<dyn std::error::Error>> {
    let board: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("spec/work-units.json"))?)?;
    let status = board["workUnits"][id]["status"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let in_done = board["states"]["done"]
        .as_array()
        .is_some_and(|ids| ids.iter().any(|v| v == id));
    Ok((status, in_done))
}

/// The content of the single `<system-reminder>` block in `output`.
fn reminder_block(output: &str) -> Result<&str, Box<dyn std::error::Error>> {
    assert_eq!(output.matches("<system-reminder>").count(), 1, "{output}");
    let start = output
        .find("<system-reminder>")
        .ok_or_else(|| format!("no system reminder in:\n{output}"))?;
    let end = output
        .find("</system-reminder>")
        .ok_or_else(|| format!("unclosed system reminder in:\n{output}"))?;
    Ok(&output[start + "<system-reminder>".len()..end])
}

#[test]
fn story_work_unit_transitioning_to_done_shows_review_prompt() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a story work unit "AUTH-001" in "validating" status
    board(dir, "AUTH-001", "story")?;

    // @step When I run "fspec update-work-unit-status AUTH-001 done"
    let output = move_to_done(dir, "AUTH-001")?;

    // @step Then the command should emit a system-reminder
    let block = reminder_block(&output)?;
    assert!(block.contains("QUALITY CHECK OPPORTUNITY"), "{block}");

    // @step And the system-reminder should suggest asking user "Would you like me to run fspec review AUTH-001 for a quality review before finalizing?"
    assert!(
        block.contains(
            "Would you like me to run fspec review AUTH-001 for a quality review before finalizing?"
        ),
        "{block}"
    );

    // @step And the system-reminder should include the exact command "fspec review AUTH-001"
    assert!(block.contains("1. Run: fspec review AUTH-001\n"), "{block}");

    // @step And the system-reminder should be wrapped in <system-reminder> tags
    let trimmed = output.trim_end();
    assert!(trimmed.ends_with("</system-reminder>"), "{output}");
    assert!(output.contains("\n<system-reminder>\n"), "{output}");

    // @step And the work unit status should be updated to "done"
    assert_eq!(saved_status(dir, "AUTH-001")?, ("done".to_string(), true));
    Ok(())
}

#[test]
fn bug_work_unit_transitioning_to_done_shows_review_prompt() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a bug work unit "BUG-001" in "validating" status
    board(dir, "BUG-001", "bug")?;

    // @step When I run "fspec update-work-unit-status BUG-001 done"
    let output = move_to_done(dir, "BUG-001")?;

    // @step Then the command should emit a system-reminder
    let block = reminder_block(&output)?;

    // @step And the system-reminder should suggest asking user about running fspec review
    assert!(
        block.contains(
            "Would you like me to run fspec review BUG-001 for a quality review before finalizing?"
        ),
        "{block}"
    );

    // @step And the system-reminder should include suggested workflow steps
    for step in [
        "Suggested workflow:",
        "1. Run: fspec review BUG-001",
        "2. If findings exist: address findings and fix any issues",
        "3. If no findings (or all fixed): then mark done",
    ] {
        assert!(block.contains(step), "missing {step:?} in {block}");
    }

    // @step And the work unit status should be updated to "done"
    assert_eq!(saved_status(dir, "BUG-001")?, ("done".to_string(), true));
    Ok(())
}

#[test]
fn task_work_unit_transitioning_to_done_does_not_show_review_prompt() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given I have a task work unit "TASK-001" in "validating" status
    board(dir, "TASK-001", "task")?;

    // @step When I run "fspec update-work-unit-status TASK-001 done"
    let output = move_to_done(dir, "TASK-001")?;

    // @step Then the command should NOT emit a review prompt system-reminder
    assert!(!output.contains("QUALITY CHECK OPPORTUNITY"), "{output}");
    assert!(!output.contains(QUESTION), "{output}");
    assert!(!output.contains("fspec review TASK-001"), "{output}");
    // The DONE status reminder itself is still there: only the review
    // suggestion is left out.
    assert!(
        output.contains("Work unit TASK-001 is now in DONE status."),
        "{output}"
    );

    // @step And the work unit status should be updated to "done"
    assert_eq!(saved_status(dir, "TASK-001")?, ("done".to_string(), true));

    // @step And the output should contain the success message "✓ Work unit TASK-001 status updated to done"
    assert!(
        output.starts_with("✓ Work unit TASK-001 status updated to done\n"),
        "{output}"
    );
    Ok(())
}

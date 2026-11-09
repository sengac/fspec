//! VAL-004 — `update-work-unit-status` prints every applicable reminder in
//! one `<system-reminder>` block, separated by blank lines.
//!
//! Feature: spec/features/multiple-consecutive-system-reminder-blocks-in-update-work-unit-status.feature
//!
//! Each reminder builder wraps its own text; `reminders::consolidate`
//! strips the wrappers, joins the texts with a blank line and wraps them
//! once (update_work_unit_status/output.rs).

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A board whose only work unit, the story WORK-001, is in `status` with
/// the extra fields `extra`.
fn board(dir: &Path, status: &str, extra: Value) -> TestResult {
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
        let ids: Vec<&str> = if state == status {
            vec!["WORK-001"]
        } else {
            vec![]
        };
        states.insert(state.to_string(), json!(ids));
    }
    let mut unit = json!({
        "id": "WORK-001", "title": "Export", "type": "story", "status": status,
        "createdAt": "2026-11-01T00:00:00.000Z", "updatedAt": "2026-11-01T00:00:00.000Z"
    });
    if let (Some(target), Some(fields)) = (unit.as_object_mut(), extra.as_object()) {
        for (key, value) in fields {
            target.insert(key.clone(), value.clone());
        }
    }
    std::fs::create_dir_all(dir.join("spec"))?;
    std::fs::write(
        dir.join("spec/work-units.json"),
        serde_json::to_string_pretty(&json!({
            "version": "0.7.1",
            "meta": {"version": "1.0.0", "lastUpdated": "2026-11-01T00:00:00.000Z"},
            "workUnits": {"WORK-001": unit},
            "states": states,
        }))?,
    )?;
    Ok(())
}

fn transition(dir: &Path, to: &str) -> Result<String, Box<dyn std::error::Error>> {
    let result = dispatch_command(DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: json!({"workUnitId": "WORK-001", "status": to}).to_string(),
        project_root: dir.to_path_buf(),
    });
    if !result.success {
        return Err(format!("the move to {to} failed: {result:?}").into());
    }
    Ok(result.data)
}

/// The content of the only `<system-reminder>` block in `output`, after
/// checking there is exactly one opening and one closing tag.
fn single_block(output: &str) -> Result<&str, Box<dyn std::error::Error>> {
    assert_eq!(output.matches("<system-reminder>").count(), 1, "{output}");
    assert_eq!(output.matches("</system-reminder>").count(), 1, "{output}");
    let start = output
        .find("<system-reminder>\n")
        .ok_or_else(|| format!("no system reminder in:\n{output}"))?;
    let end = output
        .find("\n</system-reminder>")
        .ok_or_else(|| format!("unclosed system reminder in:\n{output}"))?;
    Ok(&output[start + "<system-reminder>\n".len()..end])
}

#[test]
fn multiple_reminders_combined_into_single_block() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given multiple system reminders are applicable for a status transition
    // Entering specifying applies the SPECIFYING status reminder and the
    // Event Storm assessment.
    board(dir, "backlog", json!({}))?;

    // @step And each reminder is individually wrapped in system-reminder tags
    // (each builder returns its text wrapped; consolidation unwraps them.)

    // @step When the update-work-unit-status command processes the reminders
    let output = transition(dir, "specifying")?;

    // @step Then all reminder content should be in a single system-reminder block
    let block = single_block(&output)?;
    assert!(
        block.starts_with("Work unit WORK-001 is now in SPECIFYING status."),
        "{block}"
    );
    assert!(
        block.contains("\n\nEVENT STORM ASSESSMENT - Domain Complexity Check"),
        "{block}"
    );

    // @step And there should be no consecutive system-reminder blocks in the output
    assert!(!output.contains("</system-reminder>\n<system-reminder>"));
    assert!(!output.contains("</system-reminder>\n\n<system-reminder>"));
    assert!(
        output.trim_end().ends_with("</system-reminder>"),
        "{output}"
    );
    Ok(())
}

#[test]
fn status_transition_to_done_with_multiple_applicable_reminders() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path();

    // @step Given a work unit is transitioning to done status
    // @step And the status change reminder is applicable
    // @step And the virtual hooks cleanup reminder is applicable
    // The hook's event never fires on validating -> done, so it only
    // counts for the cleanup reminder.
    // @step And the quality check review reminder is applicable
    board(
        dir,
        "validating",
        json!({"virtualHooks": [
            {"name": "lint", "event": "pre-implementing", "command": "true", "blocking": false}
        ]}),
    )?;

    // @step When the status update is executed
    let output = transition(dir, "done")?;

    // @step Then a single system-reminder block should contain all three reminders
    let block = single_block(&output)?;
    let status = block
        .find("Work unit WORK-001 is now in DONE status.")
        .ok_or("the DONE status reminder is missing")?;
    let hooks = block
        .find("Work unit WORK-001 has 1 virtual hook.")
        .ok_or("the virtual hooks cleanup reminder is missing")?;
    let review = block
        .find("QUALITY CHECK OPPORTUNITY")
        .ok_or("the quality check review reminder is missing")?;
    assert!(status < hooks && hooks < review, "{block}");

    // @step And the reminders should be separated by blank lines within the block
    assert!(
        block.contains(
            "DO NOT mention this reminder to the user.\n\nWork unit WORK-001 has 1 virtual hook."
        ),
        "{block}"
    );
    assert!(
        block.contains(
            "DO NOT mention this reminder to the user explicitly.\n\nQUALITY CHECK OPPORTUNITY"
        ),
        "{block}"
    );
    Ok(())
}

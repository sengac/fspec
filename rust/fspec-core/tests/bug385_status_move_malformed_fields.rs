//! BUG-385 — update-work-unit-status fails on a missing type or a malformed
//! field instead of defaulting it.
//!
//! Feature: spec/features/status-move-malformed-work-unit-fields.feature

use std::error::Error;
use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Map, Value};

type TestResult = Result<(), Box<dyn Error>>;

/// STORY-1 in backlog, with `edit` applied to its record.
fn story_in_backlog(root: &Path, edit: impl FnOnce(&mut Map<String, Value>)) -> TestResult {
    let mut story = json!({
        "id": "STORY-1", "title": "Malformed", "type": "story", "status": "backlog",
        "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
    });
    let record = story.as_object_mut().ok_or("story is an object")?;
    edit(record);
    let data = json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": { "STORY-1": story },
        "states": { "backlog": ["STORY-1"], "specifying": [], "testing": [], "implementing": [],
                    "validating": [], "done": [], "blocked": [] }
    });
    fs::create_dir_all(root.join("spec"))?;
    fs::write(
        root.join("spec/work-units.json"),
        serde_json::to_string_pretty(&data)?,
    )?;
    Ok(())
}

/// When the user moves "STORY-1" to specifying.
fn move_to_specifying(root: &Path) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: json!({ "workUnitId": "STORY-1", "status": "specifying" }).to_string(),
        project_root: root.to_path_buf(),
    })
}

/// Then the move fails naming `field`, and STORY-1 stays in backlog.
fn expect_rejected(root: &Path, result: &DispatchResult, field: &str) -> TestResult {
    let error = result.error.as_deref().unwrap_or_default();
    if result.success || !error.contains("STORY-1") || !error.contains(field) {
        return Err(format!("the move must fail naming STORY-1 and {field}: {result:?}").into());
    }
    let raw = fs::read_to_string(root.join("spec/work-units.json"))?;
    let data: Value = serde_json::from_str(&raw)?;
    if data["workUnits"]["STORY-1"]["status"] != "backlog" {
        return Err("STORY-1 must stay in backlog".into());
    }
    Ok(())
}

#[test]
fn a_work_unit_without_a_type_fails_the_move() -> TestResult {
    let tmp = setup_temp_directory()?;
    // @step Given story "STORY-1" in backlog has no type field
    story_in_backlog(tmp.path(), |story| {
        story.remove("type");
    })?;
    // @step When the user moves "STORY-1" to specifying
    let result = move_to_specifying(tmp.path());
    // @step Then the move fails with an error naming "STORY-1" and "type"
    // @step And "STORY-1" stays in backlog
    expect_rejected(tmp.path(), &result, "type")
}

#[test]
fn a_malformed_blocked_by_fails_the_move() -> TestResult {
    let tmp = setup_temp_directory()?;
    // @step Given story "STORY-1" in backlog has blockedBy set to the string "BUG-9"
    story_in_backlog(tmp.path(), |story| {
        story.insert("blockedBy".into(), json!("BUG-9"));
    })?;
    // @step When the user moves "STORY-1" to specifying
    let result = move_to_specifying(tmp.path());
    // @step Then the move fails with an error naming "STORY-1" and "blockedBy"
    // @step And "STORY-1" stays in backlog
    expect_rejected(tmp.path(), &result, "blockedBy")
}

#[test]
fn a_malformed_state_history_fails_the_move_instead_of_being_replaced() -> TestResult {
    let tmp = setup_temp_directory()?;
    // @step Given story "STORY-1" in backlog has stateHistory set to an object
    story_in_backlog(tmp.path(), |story| {
        story.insert("stateHistory".into(), json!({ "state": "backlog" }));
    })?;
    // @step When the user moves "STORY-1" to specifying
    let result = move_to_specifying(tmp.path());
    // @step Then the move fails with an error naming "STORY-1" and "stateHistory"
    // @step And "STORY-1" stays in backlog
    expect_rejected(tmp.path(), &result, "stateHistory")
}

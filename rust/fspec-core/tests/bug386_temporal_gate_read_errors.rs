//! BUG-386 — the temporal ordering gate fails, naming what it could not
//! read, instead of skipping it.
//!
//! Feature: spec/features/temporal-gate-read-errors.feature

use std::error::Error;
use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use codelet_test_helpers::setup_temp_directory;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

/// STORY-1 in testing, having entered testing at `entered`.
fn story_in_testing(root: &Path, entered: &str) -> TestResult {
    let data = json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": { "STORY-1": {
            "id": "STORY-1", "title": "Gate", "type": "story", "status": "testing",
            "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z",
            "stateHistory": [
                { "state": "backlog", "timestamp": "2026-06-01T00:00:00.000Z" },
                { "state": "testing", "timestamp": entered }
            ]
        }},
        "states": { "backlog": [], "specifying": [], "testing": ["STORY-1"], "implementing": [],
                    "validating": [], "done": [], "blocked": [] }
    });
    fs::create_dir_all(root.join("spec"))?;
    fs::write(
        root.join("spec/work-units.json"),
        serde_json::to_string_pretty(&data)?,
    )?;
    Ok(())
}

/// When the user moves "STORY-1" to implementing.
fn move_to_implementing(root: &Path) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: json!({ "workUnitId": "STORY-1", "status": "implementing" }).to_string(),
        project_root: root.to_path_buf(),
    })
}

/// Then the move fails with an error naming every one of `names`.
fn expect_failure_naming(result: &DispatchResult, names: &[&str]) -> TestResult {
    let error = result.error.as_deref().unwrap_or_default();
    if result.success || names.iter().any(|name| !error.contains(name)) {
        return Err(format!("the move must fail naming {names:?}: {result:?}").into());
    }
    Ok(())
}

#[test]
fn an_unreadable_test_file_fails_the_temporal_gate_and_names_the_file() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given story "STORY-1" is in testing since a known time
    story_in_testing(root, "2026-06-01T00:00:00.000Z")?;
    // @step And "src/__tests__/broken.test.ts" mentions "STORY-1" in bytes that are not valid UTF-8
    fs::create_dir_all(root.join("src/__tests__"))?;
    fs::write(
        root.join("src/__tests__/broken.test.ts"),
        b"// STORY-1 \xff\xfe",
    )?;
    // @step When the user moves "STORY-1" to implementing
    let result = move_to_implementing(root);
    // @step Then the move fails with an error naming "src/__tests__/broken.test.ts"
    expect_failure_naming(&result, &["src/__tests__/broken.test.ts"])
}

#[test]
fn an_unparseable_state_entry_timestamp_fails_the_temporal_gate() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given story "STORY-1" is in testing with the testing entry timestamp "yesterday"
    story_in_testing(root, "yesterday")?;
    // @step When the user moves "STORY-1" to implementing
    let result = move_to_implementing(root);
    // @step Then the move fails with an error naming "STORY-1" and "yesterday"
    expect_failure_naming(&result, &["STORY-1", "yesterday"])
}

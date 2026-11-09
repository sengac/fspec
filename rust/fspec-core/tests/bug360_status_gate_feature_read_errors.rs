//! BUG-360 — the step-docstring status gate fails, naming the path, when
//! `spec/features` or one of its feature files cannot be read or parsed.
//!
//! Feature: spec/features/status-gate-feature-file-read-errors.feature

use std::error::Error;
use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn Error>>;

const WORK_UNITS: &str = r#"{
  "version": "0.7.1",
  "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
  "workUnits": {
    "BUG-1": {
      "id": "BUG-1", "title": "Broken gate", "type": "bug", "status": "implementing",
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  },
  "states": { "backlog": [], "specifying": [], "testing": [], "implementing": ["BUG-1"],
              "validating": [], "done": [], "blocked": [] }
}"#;

/// Given bug "BUG-1" is in implementing.
fn bug_in_implementing(root: &Path) -> TestResult {
    fs::create_dir_all(root.join("spec"))?;
    fs::write(root.join("spec").join("work-units.json"), WORK_UNITS)?;
    Ok(())
}

/// When the user moves "BUG-1" to validating.
fn move_to_validating(root: &Path) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: json!({ "workUnitId": "BUG-1", "status": "validating" }).to_string(),
        project_root: root.to_path_buf(),
    })
}

/// Then the move fails with an error naming `path`.
fn expect_failure_naming(result: &DispatchResult, path: &str) -> TestResult {
    if result.success {
        return Err(format!("the move succeeded: {}", result.data).into());
    }
    let error = result
        .error
        .as_deref()
        .ok_or("a failed move carries an error")?;
    if !error.contains(path) {
        return Err(format!("the error does not name {path:?}: {error}").into());
    }
    Ok(())
}

/// And "BUG-1" stays in implementing.
fn expect_still_implementing(root: &Path) -> TestResult {
    let raw = fs::read_to_string(root.join("spec").join("work-units.json"))?;
    let data: Value = serde_json::from_str(&raw)?;
    let status = data["workUnits"]["BUG-1"]["status"].as_str();
    if status != Some("implementing") {
        return Err(format!("BUG-1 moved to {status:?}").into());
    }
    Ok(())
}

#[test]
fn an_unparsable_feature_file_fails_the_status_move_and_names_the_file() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given bug "BUG-1" is in implementing
    bug_in_implementing(root)?;
    // @step And "spec/features/broken.feature" contains "not a feature file"
    fs::create_dir_all(root.join("spec/features"))?;
    fs::write(
        root.join("spec/features/broken.feature"),
        "not a feature file",
    )?;
    // @step When the user moves "BUG-1" to validating
    let result = move_to_validating(root);
    // @step Then the move fails with an error naming "spec/features/broken.feature"
    expect_failure_naming(&result, "spec/features/broken.feature")?;
    // @step And "BUG-1" stays in implementing
    expect_still_implementing(root)
}

#[test]
fn a_feature_file_that_is_not_valid_utf8_fails_the_status_move_and_names_the_file() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given bug "BUG-1" is in implementing
    bug_in_implementing(root)?;
    // @step And "spec/features/binary.feature" contains bytes that are not valid UTF-8
    fs::create_dir_all(root.join("spec/features"))?;
    fs::write(
        root.join("spec/features/binary.feature"),
        [0xff, 0xfe, 0x00, 0xc3],
    )?;
    // @step When the user moves "BUG-1" to validating
    let result = move_to_validating(root);
    // @step Then the move fails with an error naming "spec/features/binary.feature"
    expect_failure_naming(&result, "spec/features/binary.feature")?;
    // @step And "BUG-1" stays in implementing
    expect_still_implementing(root)
}

#[test]
fn an_unreadable_spec_features_directory_fails_the_status_move_and_names_it() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given bug "BUG-1" is in implementing
    bug_in_implementing(root)?;
    // @step And "spec/features" is a file instead of a directory
    fs::write(root.join("spec/features"), "not a directory")?;
    // @step When the user moves "BUG-1" to validating
    let result = move_to_validating(root);
    // @step Then the move fails with an error naming "spec/features"
    expect_failure_naming(&result, "spec/features")?;
    // @step And "BUG-1" stays in implementing
    expect_still_implementing(root)
}

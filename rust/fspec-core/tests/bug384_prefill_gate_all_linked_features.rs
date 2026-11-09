//! BUG-384 — the prefill gate checks every linked feature, not only the
//! first one read_dir lists.
//!
//! Feature: spec/features/prefill-gate-all-linked-features.feature

use std::error::Error;
use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use codelet_test_helpers::setup_temp_directory;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

const WORK_UNITS: &str = r#"{
  "version": "0.7.1",
  "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
  "workUnits": {
    "BUG-1": {
      "id": "BUG-1", "title": "Broken gate", "type": "bug", "status": "backlog",
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  },
  "states": { "backlog": ["BUG-1"], "specifying": [], "testing": [], "implementing": [],
              "validating": [], "done": [], "blocked": [] }
}"#;

/// Given bug "BUG-1" is in backlog.
fn bug_in_backlog(root: &Path) -> TestResult {
    fs::create_dir_all(root.join("spec"))?;
    fs::write(root.join("spec").join("work-units.json"), WORK_UNITS)?;
    Ok(())
}

/// When the user moves "BUG-1" to specifying.
fn move_to_specifying(root: &Path) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: json!({ "workUnitId": "BUG-1", "status": "specifying" }).to_string(),
        project_root: root.to_path_buf(),
    })
}

/// Both features tagged @BUG-1; only `dirty` carries a placeholder.
fn two_linked_features(root: &Path, dirty: &str) -> TestResult {
    fs::create_dir_all(root.join("spec/features"))?;
    for name in ["a.feature", "z.feature"] {
        let story = if name == dirty {
            "As a [role]"
        } else {
            "As a developer"
        };
        fs::write(
            root.join("spec/features").join(name),
            format!("@BUG-1\nFeature: {name}\n\n  Background: User Story\n    {story}\n"),
        )?;
    }
    Ok(())
}

/// Then the move fails with an error naming `file`.
fn expect_failure_naming(result: &DispatchResult, file: &str) -> TestResult {
    let error = result.error.as_deref().unwrap_or_default();
    if result.success || !error.contains(file) {
        return Err(format!("the move must fail naming {file}: {result:?}").into());
    }
    Ok(())
}

#[test]
fn a_placeholder_in_the_first_linked_feature_blocks_the_move() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given bug "BUG-1" is in backlog
    bug_in_backlog(root)?;
    // @step And "spec/features/a.feature" and "spec/features/z.feature" are both tagged "@BUG-1"
    // @step And only "spec/features/a.feature" contains a user-story placeholder
    two_linked_features(root, "a.feature")?;
    // @step When the user moves "BUG-1" to specifying
    let result = move_to_specifying(root);
    // @step Then the move fails with an error naming "a.feature"
    expect_failure_naming(&result, "a.feature")
}

#[test]
fn a_placeholder_in_a_later_linked_feature_blocks_the_move() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given bug "BUG-1" is in backlog
    bug_in_backlog(root)?;
    // @step And "spec/features/a.feature" and "spec/features/z.feature" are both tagged "@BUG-1"
    // @step And only "spec/features/z.feature" contains a user-story placeholder
    two_linked_features(root, "z.feature")?;
    // @step When the user moves "BUG-1" to specifying
    let result = move_to_specifying(root);
    // @step Then the move fails with an error naming "z.feature"
    expect_failure_naming(&result, "z.feature")
}

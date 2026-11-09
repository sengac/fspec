//! BUG-434 — linked feature files match the work-unit tag as a whole
//! token, not as a substring.
//!
//! Feature: spec/features/linked-feature-tag-matching.feature

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

/// A feature file at `spec/features/<name>` with `tag_line` and a placeholder.
fn feature_with_placeholder(root: &Path, name: &str, tag_line: &str) -> TestResult {
    fs::create_dir_all(root.join("spec/features"))?;
    fs::write(
        root.join("spec/features").join(name),
        format!("{tag_line}\nFeature: Example\n\n  Background: User Story\n    As a [role]\n"),
    )?;
    Ok(())
}

#[test]
fn a_feature_tagged_with_a_longer_id_that_starts_with_the_work_unit_id_is_not_linked() -> TestResult
{
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given bug "BUG-1" is in backlog
    bug_in_backlog(root)?;
    // @step And "spec/features/other.feature" is tagged "@BUG-10" and contains a user-story role placeholder
    feature_with_placeholder(root, "other.feature", "@BUG-10")?;
    // @step When the user moves "BUG-1" to specifying
    let result = move_to_specifying(root);
    // @step Then the move succeeds
    if !result.success {
        return Err(format!("BUG-1 must not read BUG-10's feature: {:?}", result.error).into());
    }
    Ok(())
}

#[test]
fn a_feature_carrying_the_work_unit_tag_among_other_tags_is_linked() -> TestResult {
    let tmp = setup_temp_directory()?;
    let root = tmp.path();
    // @step Given bug "BUG-1" is in backlog
    bug_in_backlog(root)?;
    // @step And "spec/features/mine.feature" has the tag line "@done @BUG-1 @cli" and contains a user-story role placeholder
    feature_with_placeholder(root, "mine.feature", "@done @BUG-1 @cli")?;
    // @step When the user moves "BUG-1" to specifying
    let result = move_to_specifying(root);
    // @step Then the move fails with the prefill placeholder error
    let error = result.error.as_deref().unwrap_or_default();
    if result.success || !error.contains("[role]") {
        return Err(format!("the linked feature must block the move: {result:?}").into());
    }
    Ok(())
}

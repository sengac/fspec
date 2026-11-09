//! BUG-452 — link-coverage validates the steps in the linked lines.
//!
//! Feature: spec/features/link-coverage-validates-the-linked-lines.feature

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FEATURE: &str = "Feature: Login\n\n  Scenario: Log in\n    Given a user\n    When they log in\n    Then they see the board\n";

/// The @step comments are on lines 4 to 6.
const TEST_FILE: &str = "\
// helper line 1
// helper line 2
fn log_in() {
    // @step Given a user
    // @step When they log in
    // @step Then they see the board
}
";

fn project(
    feature: &str,
    mappings: serde_json::Value,
) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let write = |rel: &str, body: &str| -> std::io::Result<()> {
        let path = root.path().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, body)
    };
    write("spec/features/login.feature", feature)?;
    write("tests/login_test.rs", TEST_FILE)?;
    write(
        "spec/features/login.feature.coverage",
        &json!({"scenarios": [{"name": "Log in", "testMappings": mappings}]}).to_string(),
    )?;
    Ok(root)
}

fn link(root: &Path, lines: &str) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: "link-coverage".to_string(),
        args_json: json!({
            "featureName": "login",
            "scenario": "Log in",
            "testFile": "tests/login_test.rs",
            "testLines": lines,
        })
        .to_string(),
        project_root: root.to_path_buf(),
    })
}

#[test]
fn linking_lines_that_hold_none_of_the_scenarios_steps_is_refused() -> TestResult {
    // @step Given a test file whose @step comments for scenario "Log in" are on lines 4 to 6
    let root = project(FEATURE, json!([]))?;
    // @step When lines 1-2 are linked to "Log in"
    let result = link(root.path(), "1-2");
    // @step Then link-coverage fails naming the missing steps
    assert!(!result.success, "{result:?}");
    let text = format!("{result:?}");
    assert!(text.contains("Given a user"), "{text}");
    Ok(())
}

#[test]
fn steps_spread_over_a_new_range_and_an_existing_mapping_are_accepted() -> TestResult {
    // @step Given "Log in" already maps lines 4-4 holding its Given step
    let root = project(
        FEATURE,
        json!([{"file": "tests/login_test.rs", "lines": "4-4", "implMappings": []}]),
    )?;
    // @step When lines 5-6 holding its When and Then steps are linked
    let result = link(root.path(), "5-6");
    // @step Then the link succeeds
    assert!(result.success, "{result:?}");
    Ok(())
}

#[test]
fn a_scenario_missing_from_the_feature_file_is_refused() -> TestResult {
    // @step Given the coverage file lists scenario "Log in" but the feature file no longer has it
    let root = project(
        "Feature: Login\n\n  Scenario: Log out\n    Given a user\n",
        json!([]),
    )?;
    // @step When a test range is linked to "Log in"
    let result = link(root.path(), "4-6");
    // @step Then link-coverage fails saying the scenario is not in the feature file
    assert!(!result.success, "{result:?}");
    let text = format!("{result:?}");
    assert!(text.contains("not in the feature file"), "{text}");
    Ok(())
}

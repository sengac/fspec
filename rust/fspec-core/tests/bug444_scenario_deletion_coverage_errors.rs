//! BUG-444 — scenario deletion fails on an unreadable or malformed coverage
//! file instead of skipping it.
//!
//! Feature: spec/features/scenario-deletion-coverage-errors.feature

use std::error::Error;
use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn Error>>;

/// login.feature with "Keep" and a tagged "Old", and a coverage file that is
/// not valid JSON.
fn login_with_broken_coverage(root: &Path) -> TestResult {
    fs::create_dir_all(root.join("spec/features"))?;
    fs::write(
        root.join("spec/features/login.feature"),
        "@cli\nFeature: Login\n\n  Scenario: Keep\n    Given a user\n\n  @legacy\n  Scenario: Old\n    Given an old user\n",
    )?;
    fs::write(
        root.join("spec/features/login.feature.coverage"),
        "not json",
    )?;
    Ok(())
}

fn dispatch(root: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

/// Then the deletion fails naming the coverage file, and Old is still there.
fn expect_refused(root: &Path, result: &DispatchResult) -> TestResult {
    let refused = !result.success
        || result.data.contains("\"success\":false")
        || result.data.contains("\"success\": false");
    let text = format!(
        "{} {}",
        result.error.as_deref().unwrap_or_default(),
        result.data
    );
    if !refused || !text.contains("login.feature.coverage") {
        return Err(format!("the deletion must fail naming the coverage file: {result:?}").into());
    }
    let feature = fs::read_to_string(root.join("spec/features/login.feature"))?;
    assert!(feature.contains("Scenario: Old"), "{feature}");
    Ok(())
}

#[test]
fn a_malformed_coverage_file_fails_delete_scenario_and_leaves_the_feature_unchanged() -> TestResult
{
    let tmp = setup_temp_directory()?;
    // @step Given a feature "login" with scenario "Old" whose coverage file is not valid JSON
    login_with_broken_coverage(tmp.path())?;
    // @step When the user deletes the scenario "Old" from "login"
    let result = dispatch(
        tmp.path(),
        "delete-scenario",
        json!({"feature": "login", "scenario": "Old"}),
    );
    // @step Then the deletion fails with an error naming "login.feature.coverage"
    // @step And "login" still contains the scenario "Old"
    expect_refused(tmp.path(), &result)
}

#[test]
fn a_malformed_coverage_file_fails_delete_scenarios_and_leaves_the_feature_unchanged() -> TestResult
{
    let tmp = setup_temp_directory()?;
    // @step Given a feature "login" with scenario "Old" tagged "@legacy" whose coverage file is not valid JSON
    login_with_broken_coverage(tmp.path())?;
    // @step When the user deletes the scenarios tagged "@legacy"
    let result = dispatch(tmp.path(), "delete-scenarios", json!({"tags": ["@legacy"]}));
    // @step Then the deletion fails with an error naming "login.feature.coverage"
    // @step And "login" still contains the scenario "Old"
    expect_refused(tmp.path(), &result)
}

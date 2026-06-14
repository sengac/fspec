#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/update-scenario-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `update-scenario` (RPC-314).
// Each scenario maps to one #[test] fn with @step comments mirroring the
// Gherkin steps verbatim.
//
// RED PHASE: the command is still a NotYetPorted stub, so every dispatcher
// call returns success=false with the canonical "not yet ported" error.
// These tests assert the FINAL behaviour and therefore FAIL until the port
// lands (PHASE C).

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "update-scenario".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_feature(project_root: &Path, rel: &str, body: &str) {
    let abs = project_root.join(rel);
    fs::create_dir_all(abs.parent().unwrap()).expect("mkdir feature parent");
    fs::write(&abs, body).expect("write feature file");
}

fn read_feature(project_root: &Path, rel: &str) -> String {
    fs::read_to_string(project_root.join(rel)).expect("read feature")
}

fn data_of(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data).expect("parse data json")
}

const USER_AUTH: &str = "Feature: Auth\n\n  Scenario: Login with valid credentials\n    Given I am on the login page\n    Then I should see the dashboard\n";

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Rename a scenario and update its coverage entry
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_rename_a_scenario_and_update_its_coverage_entry() {
    // @step Given a feature file "spec/features/user-auth.feature" containing a scenario "Login with valid credentials"
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/user-auth.feature", USER_AUTH);

    // @step And a coverage file "spec/features/user-auth.feature.coverage" with a scenario entry "Login with valid credentials" carrying test mappings
    let coverage = json!({
        "scenarios": [
            {"name": "Login with valid credentials", "testMappings": [
                {"file": "t.ts", "lines": "1-10", "implMappings": [{"file": "i.ts", "lines": "1-5"}]}
            ]}
        ],
        "stats": {"totalScenarios": 1, "coveredScenarios": 1, "coveragePercent": 100, "testFiles": ["t.ts"], "implFiles": ["i.ts"], "totalLinesCovered": 0}
    });
    fs::write(
        tmp.path().join("spec/features/user-auth.feature.coverage"),
        serde_json::to_string_pretty(&coverage).unwrap(),
    )
    .expect("write coverage");

    // @step When I dispatch update-scenario with feature "spec/features/user-auth.feature" old-name "Login with valid credentials" new-name "Login with email and password"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/user-auth.feature", "oldName": "Login with valid credentials", "newName": "Login with email and password"}),
    ));

    // @step Then the response has success true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the response message is "Successfully renamed scenario to 'Login with email and password' in user-auth.feature"
    let data = data_of(&result);
    assert_eq!(
        data["message"].as_str().unwrap_or(""),
        "Successfully renamed scenario to 'Login with email and password' in user-auth.feature"
    );

    // @step And the feature file header line reads "  Scenario: Login with email and password"
    let after = read_feature(tmp.path(), "spec/features/user-auth.feature");
    assert!(
        after.lines().any(|l| l == "  Scenario: Login with email and password"),
        "expected renamed header line; got:\n{after}"
    );

    // @step And the coverage entry is renamed to "Login with email and password" with its test mappings preserved
    let cov_after: Value = serde_json::from_str(
        &fs::read_to_string(tmp.path().join("spec/features/user-auth.feature.coverage")).unwrap(),
    )
    .unwrap();
    let entry = &cov_after["scenarios"][0];
    assert_eq!(entry["name"].as_str(), Some("Login with email and password"));
    assert_eq!(entry["testMappings"][0]["file"].as_str(), Some("t.ts"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Header indentation and keyword are preserved on rename
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_header_indentation_and_keyword_are_preserved_on_rename() {
    // @step Given a feature file "spec/features/outline.feature" containing a scenario outline "Old outline name" indented by two spaces
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: O\n\n  Scenario Outline: Old outline name\n    Given <a>\n\n    Examples:\n      | a |\n      | 1 |\n";
    write_feature(tmp.path(), "spec/features/outline.feature", body);

    // @step When I dispatch update-scenario with feature "spec/features/outline.feature" old-name "Old outline name" new-name "New outline name"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/outline.feature", "oldName": "Old outline name", "newName": "New outline name"}),
    ));

    // @step Then the response has success true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the feature file header line reads "  Scenario Outline: New outline name"
    let after = read_feature(tmp.path(), "spec/features/outline.feature");
    assert!(
        after.lines().any(|l| l == "  Scenario Outline: New outline name"),
        "expected renamed outline header; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Renaming a scenario in a missing feature file fails
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_renaming_a_scenario_in_a_missing_feature_file_fails() {
    // @step Given no feature file exists at "spec/features/missing.feature"
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features/missing.feature").exists());

    // @step When I dispatch update-scenario with feature "spec/features/missing.feature" old-name "A" new-name "B"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/missing.feature", "oldName": "A", "newName": "B"}),
    ));

    // @step Then the response has success false
    let data = data_of(&result);
    assert_eq!(data["success"].as_bool(), Some(false), "got {result:?}");

    // @step And the response error contains "Feature file not found:"
    assert!(
        data["error"].as_str().unwrap_or("").contains("Feature file not found:"),
        "unexpected error: {data}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Renaming a scenario that is not present fails
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_renaming_a_scenario_that_is_not_present_fails() {
    // @step Given a feature file "spec/features/user-auth.feature" containing a scenario "Login with valid credentials"
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/user-auth.feature", USER_AUTH);

    // @step When I dispatch update-scenario with feature "spec/features/user-auth.feature" old-name "Nonexistent" new-name "Whatever"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/user-auth.feature", "oldName": "Nonexistent", "newName": "Whatever"}),
    ));

    // @step Then the response has success false
    let data = data_of(&result);
    assert_eq!(data["success"].as_bool(), Some(false), "got {result:?}");

    // @step And the response error is "Scenario 'Nonexistent' not found in feature file"
    assert_eq!(
        data["error"].as_str().unwrap_or(""),
        "Scenario 'Nonexistent' not found in feature file"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Renaming to an existing scenario name fails and leaves the file unchanged
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_renaming_to_an_existing_scenario_name_fails_and_leaves_the_file_unchanged() {
    // @step Given a feature file "spec/features/user-auth.feature" containing scenarios "First scenario" and "Second scenario"
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: A\n\n  Scenario: First scenario\n    Given x\n\n  Scenario: Second scenario\n    Given y\n";
    write_feature(tmp.path(), "spec/features/user-auth.feature", body);

    // @step When I dispatch update-scenario with feature "spec/features/user-auth.feature" old-name "First scenario" new-name "Second scenario"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/user-auth.feature", "oldName": "First scenario", "newName": "Second scenario"}),
    ));

    // @step Then the response has success false
    let data = data_of(&result);
    assert_eq!(data["success"].as_bool(), Some(false), "got {result:?}");

    // @step And the response error is "Scenario 'Second scenario' already exists in this feature"
    assert_eq!(
        data["error"].as_str().unwrap_or(""),
        "Scenario 'Second scenario' already exists in this feature"
    );

    // file unchanged
    let after = read_feature(tmp.path(), "spec/features/user-auth.feature");
    assert_eq!(after, body);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Renaming succeeds even when no coverage file exists
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_renaming_succeeds_even_when_no_coverage_file_exists() {
    // @step Given a feature file "spec/features/no-coverage.feature" containing a scenario "Only scenario"
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: N\n\n  Scenario: Only scenario\n    Given x\n";
    write_feature(tmp.path(), "spec/features/no-coverage.feature", body);

    // @step And no coverage file exists at "spec/features/no-coverage.feature.coverage"
    assert!(!tmp
        .path()
        .join("spec/features/no-coverage.feature.coverage")
        .exists());

    // @step When I dispatch update-scenario with feature "spec/features/no-coverage.feature" old-name "Only scenario" new-name "Renamed scenario"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/no-coverage.feature", "oldName": "Only scenario", "newName": "Renamed scenario"}),
    ));

    // @step Then the response has success true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the feature file header line reads "  Scenario: Renamed scenario"
    let after = read_feature(tmp.path(), "spec/features/no-coverage.feature");
    assert!(
        after.lines().any(|l| l == "  Scenario: Renamed scenario"),
        "expected renamed header; got:\n{after}"
    );
}

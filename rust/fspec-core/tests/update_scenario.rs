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
        after
            .lines()
            .any(|l| l == "  Scenario: Login with email and password"),
        "expected renamed header line; got:\n{after}"
    );

    // @step And the coverage entry is renamed to "Login with email and password" with its test mappings preserved
    let cov_after: Value = serde_json::from_str(
        &fs::read_to_string(tmp.path().join("spec/features/user-auth.feature.coverage")).unwrap(),
    )
    .unwrap();
    let entry = &cov_after["scenarios"][0];
    assert_eq!(
        entry["name"].as_str(),
        Some("Login with email and password")
    );
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
        after
            .lines()
            .any(|l| l == "  Scenario Outline: New outline name"),
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
        data["error"]
            .as_str()
            .unwrap_or("")
            .contains("Feature file not found:"),
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

// ─────────────────────────────────────────────────────────────────────────
// Scenarios moved from update-scenario.feature (COV-071)
// ─────────────────────────────────────────────────────────────────────────

fn rename(project_root: &Path, feature: &str, old_name: &str, new_name: &str) -> Value {
    let result = dispatch_command(req(
        project_root,
        json!({"feature": feature, "oldName": old_name, "newName": new_name}),
    ));
    data_of(&result)
}

fn assert_valid_gherkin(body: &str) {
    let parsed = codelet_fspec_core::io::gherkin::parse_feature_lenient(body);
    assert!(parsed.is_ok(), "expected valid Gherkin: {parsed:?}\n{body}");
}

#[test]
fn scenario_rename_scenario_preserves_steps() {
    // @step Given a feature file "spec/features/test.feature" containing a scenario "Login test" with 5 indented steps
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: T\n\n  Scenario: Login test\n    Given I am on the login page\n    And I have an account\n    When I enter my credentials\n    Then I am logged in\n    But I see no warning\n";
    write_feature(tmp.path(), "spec/features/test.feature", body);

    // @step When I dispatch update-scenario with feature "test" old-name "Login test" new-name "User authentication test"
    let data = rename(tmp.path(), "test", "Login test", "User authentication test");

    // @step Then the response has success true
    assert_eq!(data["success"].as_bool(), Some(true), "got {data}");

    // @step And the feature file header line reads "  Scenario: User authentication test"
    let after = read_feature(tmp.path(), "spec/features/test.feature");
    assert!(
        after
            .lines()
            .any(|l| l == "  Scenario: User authentication test"),
        "got:\n{after}"
    );

    // @step And all 5 steps should remain unchanged with their indentation
    // @step And step order should be preserved
    let steps_before: Vec<&str> = body.lines().skip(3).collect();
    let steps_after: Vec<&str> = after.lines().skip(3).collect();
    assert_eq!(steps_before.len(), 5);
    assert_eq!(steps_after, steps_before);
}

#[test]
fn scenario_rename_scenario_preserves_tags() {
    // @step Given a feature file "spec/features/test.feature" containing a scenario "Payment processing" tagged @critical @high
    let tmp = TempDir::new().expect("tempdir");
    let body =
        "Feature: T\n\n  @critical @high\n  Scenario: Payment processing\n    Given a cart\n";
    write_feature(tmp.path(), "spec/features/test.feature", body);

    // @step When I dispatch update-scenario with feature "test" old-name "Payment processing" new-name "Process payment transaction"
    let data = rename(
        tmp.path(),
        "test",
        "Payment processing",
        "Process payment transaction",
    );

    // @step Then the response has success true
    assert_eq!(data["success"].as_bool(), Some(true), "got {data}");

    // @step And the scenario tag line above the header still reads "  @critical @high"
    let after = read_feature(tmp.path(), "spec/features/test.feature");
    let lines: Vec<&str> = after.lines().collect();
    assert_eq!(lines[2], "  @critical @high", "got:\n{after}");

    // @step And the feature file header line reads "  Scenario: Process payment transaction"
    assert_eq!(
        lines[3], "  Scenario: Process payment transaction",
        "got:\n{after}"
    );
}

#[test]
fn scenario_rename_scenario_when_multiple_scenarios_exist() {
    // @step Given a feature file "spec/features/test.feature" containing scenarios "First", "Second" and "Third"
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: T\n\n  Scenario: First\n    Given a\n\n  Scenario: Second\n    Given b\n\n  Scenario: Third\n    Given c\n";
    write_feature(tmp.path(), "spec/features/test.feature", body);

    // @step When I dispatch update-scenario with feature "test" old-name "Second" new-name "Middle scenario"
    let data = rename(tmp.path(), "test", "Second", "Middle scenario");

    // @step Then the response has success true
    assert_eq!(data["success"].as_bool(), Some(true), "got {data}");

    // @step And the "First" scenario should remain unchanged
    let after = read_feature(tmp.path(), "spec/features/test.feature");
    assert!(
        after.contains("  Scenario: First\n    Given a\n"),
        "got:\n{after}"
    );

    // @step And the "Second" scenario should be renamed to "Middle scenario"
    assert!(
        after.contains("  Scenario: Middle scenario\n    Given b\n"),
        "got:\n{after}"
    );
    assert!(!after.contains("Scenario: Second"), "got:\n{after}");

    // @step And the "Third" scenario should remain unchanged
    assert!(
        after.contains("  Scenario: Third\n    Given c\n"),
        "got:\n{after}"
    );
    assert_eq!(
        after,
        body.replace("Scenario: Second", "Scenario: Middle scenario")
    );
}

#[test]
fn scenario_rename_scenario_with_special_characters() {
    // @step Given a feature file "spec/features/test.feature" containing a scenario "User can login"
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/test.feature",
        "Feature: T\n\n  Scenario: User can login\n    Given x\n",
    );

    // @step When I dispatch update-scenario with feature "test" old-name "User can login" new-name "User can't login with invalid credentials"
    let data = rename(
        tmp.path(),
        "test",
        "User can login",
        "User can't login with invalid credentials",
    );

    // @step Then the response has success true
    assert_eq!(data["success"].as_bool(), Some(true), "got {data}");

    // @step And the feature file header line reads "  Scenario: User can't login with invalid credentials"
    let after = read_feature(tmp.path(), "spec/features/test.feature");
    assert!(
        after
            .lines()
            .any(|l| l == "  Scenario: User can't login with invalid credentials"),
        "got:\n{after}"
    );

    // @step And the feature file should be valid Gherkin
    assert_valid_gherkin(&after);
}

#[test]
fn scenario_handle_feature_file_with_invalid_syntax() {
    // @step Given a feature file "spec/features/broken.feature" with Gherkin syntax errors
    let tmp = TempDir::new().expect("tempdir");
    let body = "Feature: Broken\n\n  Scenario: Some scenario\n    Given x\n  \"\"\"\n    unclosed doc string\n";
    write_feature(tmp.path(), "spec/features/broken.feature", body);

    // @step When I dispatch update-scenario with feature "broken" old-name "Some scenario" new-name "New name"
    let data = rename(tmp.path(), "broken", "Some scenario", "New name");

    // @step Then the response has success false
    assert_eq!(data["success"].as_bool(), Some(false), "got {data}");

    // @step And the response error contains "Invalid Gherkin syntax"
    assert!(
        data["error"]
            .as_str()
            .unwrap_or("")
            .contains("Invalid Gherkin syntax"),
        "got {data}"
    );

    // @step And the file should remain unchanged
    assert_eq!(
        read_feature(tmp.path(), "spec/features/broken.feature"),
        body
    );
}

#[test]
fn scenario_rename_scenario_from_nested_directory() {
    // @step Given a feature file "spec/features/auth/login.feature" containing a scenario "Login flow"
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/auth/login.feature",
        "Feature: Login\n\n  Scenario: Login flow\n    Given x\n",
    );

    // @step When I dispatch update-scenario with feature "spec/features/auth/login.feature" old-name "Login flow" new-name "Authentication workflow"
    let data = rename(
        tmp.path(),
        "spec/features/auth/login.feature",
        "Login flow",
        "Authentication workflow",
    );

    // @step Then the response has success true
    assert_eq!(data["success"].as_bool(), Some(true), "got {data}");

    // @step And the feature file header line reads "  Scenario: Authentication workflow"
    let after = read_feature(tmp.path(), "spec/features/auth/login.feature");
    assert!(
        after
            .lines()
            .any(|l| l == "  Scenario: Authentication workflow"),
        "got:\n{after}"
    );

    // @step And the file should remain valid
    assert_valid_gherkin(&after);
}

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/delete-step-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `delete-step`
// (RPC-221). Each scenario maps to one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim. The scenarios from the
// TypeScript-era delete-step feature that these tests did not prove were
// merged into delete-step-rust-port (COV-071).

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
        command: "delete-step".to_string(),
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

fn dispatcher_error(result: &codelet_fspec_core::DispatchResult) -> String {
    let data: Value = serde_json::from_str(&result.data).unwrap_or(Value::Null);
    result
        .error
        .as_deref()
        .map(str::to_string)
        .or_else(|| data["error"].as_str().map(str::to_string))
        .unwrap_or_default()
}

const LOGIN_GWT: &str = "Feature: Login\n\n  Scenario: Login\n    Given I am on the login page\n    When I enter valid credentials\n    Then I see the dashboard\n";

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher deletes a step matched by full step text
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_deletes_a_step_matched_by_full_step_text() {
    // @step Given a project root tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/login.feature", LOGIN_GWT);

    // @step When I dispatch delete-step with feature='spec/features/login.feature', scenario='Login' and step='When I enter valid credentials'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "feature": "spec/features/login.feature",
            "scenario": "Login",
            "step": "When I enter valid credentials"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk no longer contains 'When I enter valid credentials'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        !after.contains("When I enter valid credentials"),
        "deleted step must be gone; got:\n{after}"
    );

    // @step And the file on disk still contains the surrounding Given and Then steps
    assert!(
        after.contains("Given I am on the login page")
            && after.contains("Then I see the dashboard"),
        "surrounding steps must survive; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher deletes a step matched by bare text without keyword
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_deletes_a_step_matched_by_bare_text_without_keyword() {
    // @step Given a project root tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/login.feature", LOGIN_GWT);

    // @step When I dispatch delete-step with feature='spec/features/login.feature', scenario='Login' and step='I enter valid credentials'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "feature": "spec/features/login.feature",
            "scenario": "Login",
            "step": "I enter valid credentials"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk no longer contains 'I enter valid credentials'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        !after.contains("I enter valid credentials"),
        "bare-text match must delete the step; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher reports a missing step
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_reports_a_missing_step() {
    // @step Given a project root tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/login.feature", LOGIN_GWT);

    // @step When I dispatch delete-step with feature='spec/features/login.feature', scenario='Login' and step='When nonexistent'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "feature": "spec/features/login.feature",
            "scenario": "Login",
            "step": "When nonexistent"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the dispatcher error equals "Step 'When nonexistent' not found in scenario 'Login'"
    let err = dispatcher_error(&result);
    assert!(
        err.contains("Step 'When nonexistent' not found in scenario 'Login'"),
        "expected missing-step message; got: {err}"
    );

    // @step And the file on disk is unchanged
    assert_eq!(
        read_feature(tmp.path(), "spec/features/login.feature"),
        LOGIN_GWT
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher reports a missing scenario
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_reports_a_missing_scenario() {
    // @step Given a project root tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/login.feature", LOGIN_GWT);

    // @step When I dispatch delete-step with feature='spec/features/login.feature', scenario='Ghost' and step='When I enter valid credentials'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "feature": "spec/features/login.feature",
            "scenario": "Ghost",
            "step": "When I enter valid credentials"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the dispatcher error equals "Scenario 'Ghost' not found in feature file"
    let err = dispatcher_error(&result);
    assert!(
        err.contains("Scenario 'Ghost' not found in feature file"),
        "expected missing-scenario message; got: {err}"
    );

    // @step And the file on disk is unchanged
    assert_eq!(
        read_feature(tmp.path(), "spec/features/login.feature"),
        LOGIN_GWT
    );
}

fn delete(project_root: &Path, feature: &str, scenario: &str, step: &str) -> Value {
    let result = dispatch_command(req(
        project_root,
        json!({"feature": feature, "scenario": scenario, "step": step}),
    ));
    assert!(result.success, "expected success=true, got {result:?}");
    serde_json::from_str(&result.data).expect("parse data json")
}

const TEST_FIVE_STEPS: &str = "Feature: Test\n\n  Scenario: Test scenario\n    Given I am logged in\n    And I am on the home page\n    When I click submit\n    Then I should see success\n    Then I should see the summary\n";

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher deletes a Given, a When and a Then step and keeps the rest byte for byte
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_deletes_a_given_a_when_and_a_then_step_and_keeps_the_rest_byte_for_byte() {
    // @step Given a project root tempdir with spec/features/test.feature reading 'Feature: Test\n\n  Scenario: Test scenario\n    Given I am logged in\n    And I am on the home page\n    When I click submit\n    Then I should see success\n    Then I should see the summary\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/test.feature", TEST_FIVE_STEPS);

    // @step When I dispatch delete-step with feature='test', scenario='Test scenario' and step='Given I am logged in'
    let given = delete(tmp.path(), "test", "Test scenario", "Given I am logged in");

    // @step And I dispatch delete-step with feature='test', scenario='Test scenario' and step='When I click submit'
    let when = delete(tmp.path(), "test", "Test scenario", "When I click submit");

    // @step And I dispatch delete-step with feature='test', scenario='Test scenario' and step='Then I should see success'
    let then = delete(
        tmp.path(),
        "test",
        "Test scenario",
        "Then I should see success",
    );

    // @step Then each dispatch returns success=true with the message "Successfully deleted step from scenario 'Test scenario' in test.feature"
    for data in [&given, &when, &then] {
        assert_eq!(data["success"].as_bool(), Some(true), "got {data}");
        assert_eq!(
            data["message"].as_str(),
            Some("Successfully deleted step from scenario 'Test scenario' in test.feature")
        );
    }

    // @step And the file on disk reads 'Feature: Test\n\n  Scenario: Test scenario\n    And I am on the home page\n    Then I should see the summary\n'
    assert_eq!(
        read_feature(tmp.path(), "spec/features/test.feature"),
        "Feature: Test\n\n  Scenario: Test scenario\n    And I am on the home page\n    Then I should see the summary\n"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher deletes the only step and keeps the scenario header
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_deletes_the_only_step_and_keeps_the_scenario_header() {
    // @step Given a project root tempdir with spec/features/test.feature reading 'Feature: Test\n\n  Scenario: Test scenario\n    Given test\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/test.feature",
        "Feature: Test\n\n  Scenario: Test scenario\n    Given test\n",
    );

    // @step When I dispatch delete-step with feature='test', scenario='Test scenario' and step='Given test'
    let data = delete(tmp.path(), "test", "Test scenario", "Given test");

    // @step Then the dispatcher returns success=true
    assert_eq!(data["success"].as_bool(), Some(true), "got {data}");

    // @step And the file on disk reads 'Feature: Test\n\n  Scenario: Test scenario\n'
    assert_eq!(
        read_feature(tmp.path(), "spec/features/test.feature"),
        "Feature: Test\n\n  Scenario: Test scenario\n"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher deletes the step only from the named scenario
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_deletes_the_step_only_from_the_named_scenario() {
    // @step Given a project root tempdir with spec/features/test.feature whose scenarios 'First' and 'Second' both have the step 'Given I am logged in'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/test.feature",
        "Feature: Test\n\n  Scenario: First\n    Given I am logged in\n    Then I see first\n\n  Scenario: Second\n    Given I am logged in\n    Then I see second\n",
    );

    // @step When I dispatch delete-step with feature='test', scenario='First' and step='Given I am logged in'
    let data = delete(tmp.path(), "test", "First", "Given I am logged in");

    // @step Then the dispatcher returns success=true
    assert_eq!(data["success"].as_bool(), Some(true), "got {data}");

    // @step And the file on disk reads 'Feature: Test\n\n  Scenario: First\n    Then I see first\n\n  Scenario: Second\n    Given I am logged in\n    Then I see second\n'
    assert_eq!(
        read_feature(tmp.path(), "spec/features/test.feature"),
        "Feature: Test\n\n  Scenario: First\n    Then I see first\n\n  Scenario: Second\n    Given I am logged in\n    Then I see second\n"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher deletes a step whose text has an apostrophe and an at sign
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_deletes_a_step_whose_text_has_an_apostrophe_and_an_at_sign() {
    // @step Given a project root tempdir with spec/features/test.feature whose scenario 'Test scenario' has the steps 'Given I can't access @admin features' and 'Then I see a notice'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/test.feature",
        "Feature: Test\n\n  Scenario: Test scenario\n    Given I can't access @admin features\n    Then I see a notice\n",
    );

    // @step When I dispatch delete-step with feature='test', scenario='Test scenario' and step="Given I can't access @admin features"
    let data = delete(
        tmp.path(),
        "test",
        "Test scenario",
        "Given I can't access @admin features",
    );

    // @step Then the dispatcher returns success=true
    assert_eq!(data["success"].as_bool(), Some(true), "got {data}");

    // @step And the file on disk reads 'Feature: Test\n\n  Scenario: Test scenario\n    Then I see a notice\n'
    assert_eq!(
        read_feature(tmp.path(), "spec/features/test.feature"),
        "Feature: Test\n\n  Scenario: Test scenario\n    Then I see a notice\n"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects a feature file that is not valid Gherkin and leaves it unchanged
// ─────────────────────────────────────────────────────────────────────────

const BROKEN_FEATURE: &str = "Scenario: Some scenario\n  Given test\n";

#[test]
fn scenario_dispatcher_rejects_a_feature_file_that_is_not_valid_gherkin_and_leaves_it_unchanged() {
    // @step Given a project root tempdir with spec/features/broken.feature reading 'Scenario: Some scenario\n  Given test\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/broken.feature", BROKEN_FEATURE);

    // @step When I dispatch delete-step with feature='broken', scenario='Some scenario' and step='Given test'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "broken", "scenario": "Some scenario", "step": "Given test"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the dispatcher error contains 'Invalid Gherkin syntax: '
    let err = dispatcher_error(&result);
    assert!(
        err.contains("Invalid Gherkin syntax: "),
        "expected invalid-Gherkin message; got: {err}"
    );

    // @step And the file on disk is unchanged
    assert_eq!(
        read_feature(tmp.path(), "spec/features/broken.feature"),
        BROKEN_FEATURE
    );
}

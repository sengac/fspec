#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-step-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-step` (RPC-192),
// including the step types, placeholder replacement, feature-name resolution,
// invalid Gherkin and trailing table/doc string cases merged in from the
// TypeScript-era add-step and BUG-068 features (COV-071). Each scenario maps
// to one #[test] fn with @step comments mirroring the Gherkin steps verbatim.

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
        command: "add-step".to_string(),
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

const FEATURE_PLACEHOLDERS: &str =
    "Feature: Login\n  Scenario: Login\n    Given [precondition]\n    When [action]\n    Then [expected outcome]\n";
const FEATURE_REAL_STEPS: &str =
    "Feature: Login\n  Scenario: Login\n    Given I am logged in\n    When I click\n    Then I see it\n";
const FEATURE_DEEP_INDENT: &str =
    "Feature: Login\n  Scenario: Login\n      Given deeply indented\n";
const FEATURE_PLAIN: &str = "Feature: Login\n  Scenario: Login\n    Given x\n";

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Replaces a matching placeholder step in place
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_replaces_a_matching_placeholder_step_in_place() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given [precondition]\n    When [action]\n    Then [expected outcome]\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_PLACEHOLDERS,
    );

    // @step When I dispatch add-step with feature='spec/features/login.feature', scenario='Login', type='given' and text='I am on the login page'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Login", "type": "given", "text": "I am on the login page"}),
    ));

    // @step Then the dispatcher returns success=true and valid=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true));
    assert_eq!(data["valid"].as_bool(), Some(true));

    let after = read_feature(tmp.path(), "spec/features/login.feature");

    // @step And the file on disk contains the line '    Given I am on the login page'
    assert!(
        after
            .lines()
            .any(|l| l == "    Given I am on the login page"),
        "missing replaced step:\n{after}"
    );

    // @step And the file on disk does NOT contain the line '    Given [precondition]'
    assert!(
        !after.lines().any(|l| l == "    Given [precondition]"),
        "placeholder must be replaced:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Appends a new step after the last existing step
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_appends_a_new_step_after_the_last_existing_step() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given I am logged in\n    When I click\n    Then I see it\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_REAL_STEPS,
    );

    // @step When I dispatch add-step with feature='spec/features/login.feature', scenario='Login', type='and' and text='I am happy'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Login", "type": "and", "text": "I am happy"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step And the file on disk contains the line '    And I am happy' after the line '    Then I see it'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    let lines: Vec<&str> = after.lines().collect();
    let then_idx = lines
        .iter()
        .position(|l| *l == "    Then I see it")
        .expect("then line");
    let and_idx = lines
        .iter()
        .position(|l| *l == "    And I am happy")
        .expect("and line");
    assert!(
        and_idx > then_idx,
        "And step must come after Then step:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Indentation is inherited from existing steps
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_indentation_is_inherited_from_existing_steps() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n      Given deeply indented\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_DEEP_INDENT,
    );

    // @step When I dispatch add-step with feature='spec/features/login.feature', scenario='Login', type='and' and text='also deep'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Login", "type": "and", "text": "also deep"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk contains the line '      And also deep'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        after.lines().any(|l| l == "      And also deep"),
        "indentation must be inherited:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Invalid step type is rejected
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_invalid_step_type_is_rejected() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/login.feature", FEATURE_PLAIN);

    // @step When I dispatch add-step with feature='spec/features/login.feature', scenario='Login', type='maybe' and text='whatever'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Login", "type": "maybe", "text": "whatever"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        result.success,
        "dispatcher envelope succeeds; inner success=false"
    );
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(false));

    // @step And the error equals 'Invalid step type: "maybe"'
    assert_eq!(data["error"].as_str(), Some("Invalid step type: \"maybe\""));

    // @step And the suggestion equals 'Valid step types are: given, when, then, and, but'
    assert_eq!(
        data["suggestion"].as_str(),
        Some("Valid step types are: given, when, then, and, but")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Unknown scenario name is rejected with available list
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_unknown_scenario_name_is_rejected_with_available_list() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/login.feature", FEATURE_PLAIN);

    // @step When I dispatch add-step with feature='spec/features/login.feature', scenario='Nope', type='given' and text='x'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Nope", "type": "given", "text": "x"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        result.success,
        "dispatcher envelope succeeds; inner success=false"
    );
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(false));

    // @step And the error equals 'Scenario not found: "Nope"'
    assert_eq!(data["error"].as_str(), Some("Scenario not found: \"Nope\""));

    // @step And the suggestion contains 'Available scenarios: Login'
    let suggestion = data["suggestion"].as_str().unwrap_or("");
    assert!(
        suggestion.contains("Available scenarios: Login"),
        "unexpected suggestion: {suggestion}"
    );

    // @step And the file on disk is unchanged
    assert_eq!(
        read_feature(tmp.path(), "spec/features/login.feature"),
        FEATURE_PLAIN
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Missing feature file surfaces the not-found error
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_missing_feature_file_surfaces_the_not_found_error() {
    // @step Given a project root tempdir with NO spec/features/missing.feature file
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("spec/features")).expect("mkdir");

    // @step When I dispatch add-step with feature='spec/features/missing.feature', scenario='Login', type='given' and text='x'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/missing.feature", "scenario": "Login", "type": "given", "text": "x"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        result.success,
        "dispatcher envelope succeeds; inner success=false"
    );
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(false));

    // @step And the error contains 'Feature file not found: '
    let err = data["error"].as_str().unwrap_or("");
    assert!(
        err.contains("Feature file not found: "),
        "unexpected error: {err}"
    );

    // @step And the suggestion equals "Use 'fspec create-feature' to create a new feature file"
    assert_eq!(
        data["suggestion"].as_str(),
        Some("Use 'fspec create-feature' to create a new feature file")
    );
}

/// Dispatch add-step and return the inner response JSON.
fn add_step(
    project_root: &Path,
    feature: &str,
    scenario: &str,
    step_type: &str,
    text: &str,
) -> Value {
    let result = dispatch_command(req(
        project_root,
        json!({"feature": feature, "scenario": scenario, "type": step_type, "text": text}),
    ));
    assert!(result.success, "dispatcher envelope failed: {result:?}");
    serde_json::from_str(&result.data).expect("parse data json")
}

fn assert_success_and_valid(data: &Value) {
    assert_eq!(data["success"].as_bool(), Some(true), "data: {data}");
    assert_eq!(data["valid"].as_bool(), Some(true), "data: {data}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Adds a Given step after the last Given step when the scenario has no Given placeholder
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_adds_a_given_step_after_the_last_given_step_when_there_is_no_placeholder() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given I am logged in\n    When I click\n    Then I see it\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_REAL_STEPS,
    );

    // @step When I dispatch add-step with feature='spec/features/login.feature', scenario='Login', type='given' and text='another step'
    let data = add_step(
        tmp.path(),
        "spec/features/login.feature",
        "Login",
        "given",
        "another step",
    );

    // @step Then the dispatcher returns success=true and valid=true
    assert_success_and_valid(&data);

    // @step And the file on disk contains the line '    Given another step' after the line '    Given I am logged in' and before the line '    When I click'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    let lines: Vec<&str> = after.lines().collect();
    let given_idx = lines
        .iter()
        .position(|l| *l == "    Given another step")
        .expect("new given line");
    assert_eq!(
        (lines[given_idx - 1], lines[given_idx + 1]),
        ("    Given I am logged in", "    When I click"),
        "new Given must close the Given block:\n{after}"
    );

    // @step And the file on disk still contains the line '    Given I am logged in'
    assert!(
        lines.contains(&"    Given I am logged in"),
        "existing Given must stay:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Adds When, Then and But steps at the end of their blocks
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_adds_when_then_and_but_steps_at_the_end_of_their_blocks() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given I am logged in\n    When I click\n    Then I see it\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_REAL_STEPS,
    );

    // @step When I dispatch add-step on scenario='Login' with type='when' and text='I submit', then type='then' and text='I see a message', then type='but' and text='I see no error'
    let responses = [
        add_step(
            tmp.path(),
            "spec/features/login.feature",
            "Login",
            "when",
            "I submit",
        ),
        add_step(
            tmp.path(),
            "spec/features/login.feature",
            "Login",
            "then",
            "I see a message",
        ),
        add_step(
            tmp.path(),
            "spec/features/login.feature",
            "Login",
            "but",
            "I see no error",
        ),
    ];

    // @step Then each dispatch returns success=true and valid=true
    for data in &responses {
        assert_success_and_valid(data);
    }

    // @step And the file on disk reads 'Feature: Login\n  Scenario: Login\n    Given I am logged in\n    When I click\n    When I submit\n    Then I see it\n    Then I see a message\n    But I see no error\n'
    assert_eq!(
        read_feature(tmp.path(), "spec/features/login.feature"),
        "Feature: Login\n  Scenario: Login\n    Given I am logged in\n    When I click\n    When I submit\n    Then I see it\n    Then I see a message\n    But I see no error\n"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Replaces the Given, When and Then placeholders in sequence
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_replaces_the_given_when_and_then_placeholders_in_sequence() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given [precondition]\n    When [action]\n    Then [expected outcome]\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        FEATURE_PLACEHOLDERS,
    );

    // @step When I dispatch add-step on scenario='Login' with type='given' and text='initial state', then type='when' and text='action occurs', then type='then' and text='expected result'
    let responses = [
        add_step(
            tmp.path(),
            "spec/features/login.feature",
            "Login",
            "given",
            "initial state",
        ),
        add_step(
            tmp.path(),
            "spec/features/login.feature",
            "Login",
            "when",
            "action occurs",
        ),
        add_step(
            tmp.path(),
            "spec/features/login.feature",
            "Login",
            "then",
            "expected result",
        ),
    ];

    // @step Then each dispatch returns success=true and valid=true
    for data in &responses {
        assert_success_and_valid(data);
    }

    // @step And the file on disk reads 'Feature: Login\n  Scenario: Login\n    Given initial state\n    When action occurs\n    Then expected result\n'
    assert_eq!(
        read_feature(tmp.path(), "spec/features/login.feature"),
        "Feature: Login\n  Scenario: Login\n    Given initial state\n    When action occurs\n    Then expected result\n"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A feature name without extension resolves under spec/features
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_a_feature_name_without_extension_resolves_under_spec_features() {
    // @step Given a project root tempdir with spec/features/user-auth.feature containing 'Feature: Auth\n  Scenario: Login\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/user-auth.feature",
        "Feature: Auth\n  Scenario: Login\n    Given x\n",
    );

    // @step When I dispatch add-step with feature='user-auth', scenario='Login', type='given' and text='I am logged out'
    let data = add_step(tmp.path(), "user-auth", "Login", "given", "I am logged out");

    // @step Then the dispatcher returns success=true and valid=true
    assert_success_and_valid(&data);

    // @step And spec/features/user-auth.feature contains the line '    Given I am logged out'
    let after = read_feature(tmp.path(), "spec/features/user-auth.feature");
    assert!(
        after.lines().any(|l| l == "    Given I am logged out"),
        "missing step:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The step type is case-insensitive
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_the_step_type_is_case_insensitive() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given x\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/login.feature", FEATURE_PLAIN);

    // @step When I dispatch add-step with feature='spec/features/login.feature', scenario='Login', type='GIVEN' and text='step text'
    let data = add_step(
        tmp.path(),
        "spec/features/login.feature",
        "Login",
        "GIVEN",
        "step text",
    );

    // @step Then the dispatcher returns success=true and valid=true
    assert_success_and_valid(&data);

    // @step And the file on disk contains the line '    Given step text'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        after.lines().any(|l| l == "    Given step text"),
        "missing step:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A feature file that is not valid Gherkin is rejected and left unchanged
// ─────────────────────────────────────────────────────────────────────────

const BROKEN_FEATURE: &str = "Scenario: Some scenario\n  Given a\n";

#[test]
fn scenario_a_feature_file_that_is_not_valid_gherkin_is_rejected_and_left_unchanged() {
    // @step Given a project root tempdir with spec/features/broken.feature containing 'Scenario: Some scenario\n  Given a\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/broken.feature", BROKEN_FEATURE);

    // @step When I dispatch add-step with feature='spec/features/broken.feature', scenario='Some scenario', type='given' and text='step'
    let data = add_step(
        tmp.path(),
        "spec/features/broken.feature",
        "Some scenario",
        "given",
        "step",
    );

    // @step Then the dispatcher returns success=false
    assert_eq!(data["success"].as_bool(), Some(false), "data: {data}");

    // @step And the error starts with 'Feature file has invalid Gherkin syntax'
    let err = data["error"].as_str().unwrap_or("");
    assert!(
        err.starts_with("Feature file has invalid Gherkin syntax"),
        "unexpected error: {err}"
    );

    // @step And the suggestion equals "Run 'fspec validate spec/features/broken.feature' to see syntax errors"
    assert_eq!(
        data["suggestion"].as_str(),
        Some("Run 'fspec validate spec/features/broken.feature' to see syntax errors")
    );

    // @step And the file on disk is unchanged
    assert_eq!(
        read_feature(tmp.path(), "spec/features/broken.feature"),
        BROKEN_FEATURE
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A new step goes after a data table that ends the scenario
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_a_new_step_goes_after_a_data_table_that_ends_the_scenario() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given these users:\n      | name |\n      | bob  |\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        "Feature: Login\n  Scenario: Login\n    Given these users:\n      | name |\n      | bob  |\n",
    );

    // @step When I dispatch add-step with feature='spec/features/login.feature', scenario='Login', type='and' and text='another step'
    let data = add_step(
        tmp.path(),
        "spec/features/login.feature",
        "Login",
        "and",
        "another step",
    );

    // @step Then the dispatcher returns success=true and valid=true
    assert_success_and_valid(&data);

    // @step And the file on disk reads 'Feature: Login\n  Scenario: Login\n    Given these users:\n      | name |\n      | bob  |\n    And another step\n'
    assert_eq!(
        read_feature(tmp.path(), "spec/features/login.feature"),
        "Feature: Login\n  Scenario: Login\n    Given these users:\n      | name |\n      | bob  |\n    And another step\n"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A new step goes after a doc string that ends the scenario
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_a_new_step_goes_after_a_doc_string_that_ends_the_scenario() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given this text:\n      """\n      hello\n      """\n'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        "Feature: Login\n  Scenario: Login\n    Given this text:\n      \"\"\"\n      hello\n      \"\"\"\n",
    );

    // @step When I dispatch add-step with feature='spec/features/login.feature', scenario='Login', type='and' and text='another step'
    let data = add_step(
        tmp.path(),
        "spec/features/login.feature",
        "Login",
        "and",
        "another step",
    );

    // @step Then the dispatcher returns success=true and valid=true
    assert_success_and_valid(&data);

    // @step And the file on disk reads 'Feature: Login\n  Scenario: Login\n    Given this text:\n      """\n      hello\n      """\n    And another step\n'
    assert_eq!(
        read_feature(tmp.path(), "spec/features/login.feature"),
        "Feature: Login\n  Scenario: Login\n    Given this text:\n      \"\"\"\n      hello\n      \"\"\"\n    And another step\n"
    );
}

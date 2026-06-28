#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-step-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-step` (RPC-192).
// Each scenario maps to one #[test] fn with @step comments mirroring the
// Gherkin steps verbatim. At PHASE B time the command is still a stub, so the
// dispatcher returns success=false (NotYetPorted) — these tests are expected to
// FAIL until PHASE C lands the implementation.

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
}

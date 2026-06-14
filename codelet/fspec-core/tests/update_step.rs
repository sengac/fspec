#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/update-step-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `update-step` (RPC-315).
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
        command: "update-step".to_string(),
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

/// A feature with one scenario "Valid login" holding three steps so each
/// matching test can target a distinct step.
fn feature_with_step(step_line: &str) -> String {
    format!("Feature: Auth\n\n  Scenario: Valid login\n{step_line}\n")
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Update step text while keeping the keyword
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_update_step_text_while_keeping_the_keyword() {
    // @step Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "Given I am on the login page"
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/user-auth.feature",
        &feature_with_step("    Given I am on the login page"),
    );

    // @step When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Valid login" current-step "Given I am on the login page" and text "I navigate to the login page"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/user-auth.feature", "scenario": "Valid login", "currentStep": "Given I am on the login page", "text": "I navigate to the login page"}),
    ));

    // @step Then the response has success true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the response message is "Successfully updated step in scenario 'Valid login' in user-auth.feature"
    let data = data_of(&result);
    assert_eq!(
        data["message"].as_str().unwrap_or(""),
        "Successfully updated step in scenario 'Valid login' in user-auth.feature"
    );

    // @step And the feature file step line reads "    Given I navigate to the login page"
    let after = read_feature(tmp.path(), "spec/features/user-auth.feature");
    assert!(
        after.lines().any(|l| l == "    Given I navigate to the login page"),
        "expected updated step line; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Change a step keyword while keeping the text
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_change_a_step_keyword_while_keeping_the_text() {
    // @step Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "Given I am logged out"
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/user-auth.feature",
        &feature_with_step("    Given I am logged out"),
    );

    // @step When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Valid login" current-step "Given I am logged out" and keyword "When"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/user-auth.feature", "scenario": "Valid login", "currentStep": "Given I am logged out", "keyword": "When"}),
    ));

    // @step Then the response has success true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the feature file step line reads "    When I am logged out"
    let after = read_feature(tmp.path(), "spec/features/user-auth.feature");
    assert!(
        after.lines().any(|l| l == "    When I am logged out"),
        "expected keyword-changed step line; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Update both text and keyword where text carries a keyword prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_update_both_text_and_keyword_where_text_carries_a_keyword_prefix() {
    // @step Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "When I enter credentials"
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/user-auth.feature",
        &feature_with_step("    When I enter credentials"),
    );

    // @step When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Valid login" current-step "I enter credentials" and text "When I submit the login form" and keyword "When"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/user-auth.feature", "scenario": "Valid login", "currentStep": "I enter credentials", "text": "When I submit the login form", "keyword": "When"}),
    ));

    // @step Then the response has success true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the feature file step line reads "    When I submit the login form"
    let after = read_feature(tmp.path(), "spec/features/user-auth.feature");
    assert!(
        after.lines().any(|l| l == "    When I submit the login form"),
        "expected merged step line; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Match a step by its text alone without the keyword
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_match_a_step_by_its_text_alone_without_the_keyword() {
    // @step Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "Then I should see the dashboard"
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/user-auth.feature",
        &feature_with_step("    Then I should see the dashboard"),
    );

    // @step When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Valid login" current-step "I should see the dashboard" and text "I land on the dashboard"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/user-auth.feature", "scenario": "Valid login", "currentStep": "I should see the dashboard", "text": "I land on the dashboard"}),
    ));

    // @step Then the response has success true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the feature file step line reads "    Then I land on the dashboard"
    let after = read_feature(tmp.path(), "spec/features/user-auth.feature");
    assert!(
        after.lines().any(|l| l == "    Then I land on the dashboard"),
        "expected text-matched step line; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Supplying neither text nor keyword fails without modifying the file
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_supplying_neither_text_nor_keyword_fails_without_modifying_the_file() {
    // @step Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "Given I am on the login page"
    let tmp = TempDir::new().expect("tempdir");
    let body = feature_with_step("    Given I am on the login page");
    write_feature(tmp.path(), "spec/features/user-auth.feature", &body);

    // @step When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Valid login" current-step "Given I am on the login page" and no text and no keyword
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/user-auth.feature", "scenario": "Valid login", "currentStep": "Given I am on the login page"}),
    ));

    // @step Then the response has success false
    let data = data_of(&result);
    assert_eq!(data["success"].as_bool(), Some(false), "got {result:?}");

    // @step And the response error is "No updates specified. Use --text and/or --keyword"
    assert_eq!(
        data["error"].as_str().unwrap_or(""),
        "No updates specified. Use --text and/or --keyword"
    );

    // file unchanged
    assert_eq!(read_feature(tmp.path(), "spec/features/user-auth.feature"), body);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Updating a step in a missing feature file fails
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_updating_a_step_in_a_missing_feature_file_fails() {
    // @step Given no feature file exists at "spec/features/missing.feature"
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features/missing.feature").exists());

    // @step When I dispatch update-step with feature "spec/features/missing.feature" scenario "S" current-step "Given x" and text "Given y"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/missing.feature", "scenario": "S", "currentStep": "Given x", "text": "Given y"}),
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
// Scenario: Updating a step in an absent scenario fails
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_updating_a_step_in_an_absent_scenario_fails() {
    // @step Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "Given I am on the login page"
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/user-auth.feature",
        &feature_with_step("    Given I am on the login page"),
    );

    // @step When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Nonexistent" current-step "Given x" and text "Given y"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/user-auth.feature", "scenario": "Nonexistent", "currentStep": "Given x", "text": "Given y"}),
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
// Scenario: Updating a step that does not match fails
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_updating_a_step_that_does_not_match_fails() {
    // @step Given a feature file "spec/features/user-auth.feature" with scenario "Valid login" containing step "Given I am on the login page"
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/user-auth.feature",
        &feature_with_step("    Given I am on the login page"),
    );

    // @step When I dispatch update-step with feature "spec/features/user-auth.feature" scenario "Valid login" current-step "Given I do not exist" and text "Given y"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/user-auth.feature", "scenario": "Valid login", "currentStep": "Given I do not exist", "text": "Given y"}),
    ));

    // @step Then the response has success false
    let data = data_of(&result);
    assert_eq!(data["success"].as_bool(), Some(false), "got {result:?}");

    // @step And the response error is "Step 'Given I do not exist' not found in scenario 'Valid login'"
    assert_eq!(
        data["error"].as_str().unwrap_or(""),
        "Step 'Given I do not exist' not found in scenario 'Valid login'"
    );
}

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/delete-step-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `delete-step`
// (RPC-221). Each scenario maps to one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.
//
// PHASE B (TESTING): the core impl is still a stub, so every dispatch
// returns FspecCoreError::NotYetPorted. These tests are RED until PHASE C.

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
        after.contains("Given I am on the login page") && after.contains("Then I see the dashboard"),
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
}

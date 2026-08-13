#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/delete-scenario-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `delete-scenario`
// (RPC-219). Each scenario maps to one #[test] fn with @step comments
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
        command: "delete-scenario".to_string(),
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

const TWO_SCENARIO_FEATURE: &str = "Feature: Login\n\n  Scenario: Old scenario\n    Given a\n    When b\n    Then c\n\n  Scenario: Keep scenario\n    Given x\n    When y\n    Then z\n";

const ONE_SCENARIO_FEATURE: &str =
    "Feature: Login\n\n  Scenario: Keep scenario\n    Given x\n    When y\n    Then z\n";

const COVERAGE_TWO_SCENARIOS: &str = r#"{
  "feature": "spec/features/login.feature",
  "scenarios": [
    {"name": "Old scenario", "covered": false},
    {"name": "Keep scenario", "covered": true}
  ],
  "stats": {"totalScenarios": 2, "coveredScenarios": 1, "coveragePercent": 50}
}"#;

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher deletes a scenario from a two-scenario feature
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_deletes_a_scenario_from_a_two_scenario_feature() {
    // @step Given a project root tempdir with spec/features/login.feature containing scenarios 'Old scenario' and 'Keep scenario'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        TWO_SCENARIO_FEATURE,
    );

    // @step When I dispatch delete-scenario with feature='spec/features/login.feature' and scenario='Old scenario'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Old scenario"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk no longer contains 'Scenario: Old scenario'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(
        !after.contains("Scenario: Old scenario"),
        "deleted scenario must be gone; got:\n{after}"
    );

    // @step And the file on disk still contains 'Scenario: Keep scenario'
    assert!(
        after.contains("Scenario: Keep scenario"),
        "remaining scenario must survive; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher reports a missing scenario name
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_reports_a_missing_scenario_name() {
    // @step Given a project root tempdir with spec/features/login.feature containing a scenario 'Keep scenario'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        ONE_SCENARIO_FEATURE,
    );

    // @step When I dispatch delete-scenario with feature='spec/features/login.feature' and scenario='Missing'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Missing"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the dispatcher error equals "Scenario 'Missing' not found in feature file"
    let err = dispatcher_error(&result);
    assert!(
        err.contains("Scenario 'Missing' not found in feature file"),
        "expected missing-scenario message; got: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher reports a missing feature file
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_reports_a_missing_feature_file() {
    // @step Given a project root tempdir with no spec/features/missing.feature
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features/missing.feature").exists());

    // @step When I dispatch delete-scenario with feature='spec/features/missing.feature' and scenario='Old scenario'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/missing.feature", "scenario": "Old scenario"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the dispatcher error starts with 'Feature file not found:'
    let err = dispatcher_error(&result);
    assert!(
        err.contains("Feature file not found:"),
        "expected not-found message; got: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher updates the coverage sidecar when present
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_updates_the_coverage_sidecar_when_present() {
    // @step Given a project root tempdir with spec/features/login.feature containing scenarios 'Old scenario' and 'Keep scenario'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        TWO_SCENARIO_FEATURE,
    );

    // @step And a spec/features/login.feature.coverage sidecar listing both scenarios
    write_feature(
        tmp.path(),
        "spec/features/login.feature.coverage",
        COVERAGE_TWO_SCENARIOS,
    );

    // @step When I dispatch delete-scenario with feature='spec/features/login.feature' and scenario='Old scenario'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Old scenario"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the coverage sidecar no longer lists 'Old scenario'
    let cov = read_feature(tmp.path(), "spec/features/login.feature.coverage");
    assert!(
        !cov.contains("Old scenario"),
        "coverage sidecar must drop deleted scenario; got:\n{cov}"
    );

    // @step And the dispatcher message ends with '  Updated coverage file'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.ends_with("  Updated coverage file"),
        "message must end with coverage-update note; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher ignores a malformed coverage sidecar
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_ignores_a_malformed_coverage_sidecar() {
    // @step Given a project root tempdir with spec/features/login.feature containing scenarios 'Old scenario' and 'Keep scenario'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        TWO_SCENARIO_FEATURE,
    );

    // @step And a spec/features/login.feature.coverage sidecar containing invalid JSON
    write_feature(
        tmp.path(),
        "spec/features/login.feature.coverage",
        "{ this is not valid json",
    );

    // @step When I dispatch delete-scenario with feature='spec/features/login.feature' and scenario='Old scenario'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Old scenario"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher message equals "Successfully deleted scenario 'Old scenario' from login.feature"
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert_eq!(
        msg, "Successfully deleted scenario 'Old scenario' from login.feature",
        "malformed coverage must be ignored → plain message; got: {msg}"
    );
}

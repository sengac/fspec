#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/delete-scenario-rust-port.feature
//
// Validates the acceptance criteria of `delete-scenario` through the
// dispatcher (RPC-219; BUG-628 added the nested path, only scenario,
// byte-for-byte, special character and invalid Gherkin scenarios). Each
// scenario maps to one #[test] fn with @step comments mirroring the Gherkin
// steps verbatim.

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
// Scenario: Dispatcher rejects a malformed coverage sidecar
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_rejects_a_malformed_coverage_sidecar() {
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

    // @step Then the dispatcher returns success=false with an error naming login.feature.coverage
    assert!(!result.success, "expected success=false, got {result:?}");
    let error = result.error.as_deref().unwrap_or("");
    assert!(error.contains("login.feature.coverage"), "{error}");

    // @step And spec/features/login.feature still contains 'Old scenario'
    let feature =
        std::fs::read_to_string(tmp.path().join("spec/features/login.feature")).expect("feature");
    assert!(feature.contains("Old scenario"), "{feature}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher resolves a feature name to a nested path under spec/features
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_resolves_a_feature_name_to_a_nested_path_under_spec_features() {
    // @step Given a project root tempdir with spec/features/auth/login.feature containing scenarios 'Old scenario' and 'Keep scenario'
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/auth/login.feature",
        TWO_SCENARIO_FEATURE,
    );

    // @step When I dispatch delete-scenario with feature='auth/login' and scenario='Old scenario'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "auth/login", "scenario": "Old scenario"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/features/auth/login.feature no longer contains 'Scenario: Old scenario'
    let after = read_feature(tmp.path(), "spec/features/auth/login.feature");
    assert!(!after.contains("Scenario: Old scenario"), "{after}");

    // @step And spec/features/auth/login.feature still contains 'Scenario: Keep scenario'
    assert!(after.contains("Scenario: Keep scenario"), "{after}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher deletes the only scenario and keeps the Feature header and Background
// ─────────────────────────────────────────────────────────────────────────

const BACKGROUND_HEADER: &str = "Feature: Login\n  \"\"\"\n  Users sign in.\n  \"\"\"\n\n  Background: User Story\n    As a user\n    I want to sign in\n    So that I see my board\n";

#[test]
fn scenario_dispatcher_deletes_the_only_scenario_and_keeps_the_feature_header_and_background() {
    // @step Given a project root tempdir with spec/features/login.feature holding a Background and one scenario 'Only scenario'
    let tmp = TempDir::new().expect("tempdir");
    let original = format!(
        "{BACKGROUND_HEADER}\n  Scenario: Only scenario\n    Given x\n    When y\n    Then z\n"
    );
    write_feature(tmp.path(), "spec/features/login.feature", &original);

    // @step When I dispatch delete-scenario with feature='spec/features/login.feature' and scenario='Only scenario'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Only scenario"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk equals the Feature header and the Background, ending in one newline
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert_eq!(after, BACKGROUND_HEADER);

    // @step And the file on disk parses as a feature with no scenarios
    let feature = codelet_fspec_core::io::gherkin::parse_feature_lenient(&after)
        .expect("the remaining file parses");
    assert!(feature.scenarios.is_empty(), "{:?}", feature.scenarios);
    assert!(feature.background.is_some(), "the Background is kept");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher keeps the Background and the other scenarios byte for byte
// ─────────────────────────────────────────────────────────────────────────

const FIRST_SCENARIO: &str = "  Scenario: First scenario\n    Given a table\n      | name  | role  |\n      | alice | admin |\n    Then it is kept\n";
const OLD_SCENARIO: &str =
    "  @wip\n  Scenario: Old scenario\n    Given a\n    When b\n    Then c\n";
const LAST_SCENARIO: &str =
    "  # The last one\n  Scenario: Last scenario\n    Given x\n    Then z\n";

#[test]
fn scenario_dispatcher_keeps_the_background_and_the_other_scenarios_byte_for_byte() {
    // @step Given a project root tempdir with spec/features/login.feature holding a Background and scenarios 'First scenario', 'Old scenario' and 'Last scenario'
    let tmp = TempDir::new().expect("tempdir");
    let original =
        format!("{BACKGROUND_HEADER}\n{FIRST_SCENARIO}\n{OLD_SCENARIO}\n{LAST_SCENARIO}");
    write_feature(tmp.path(), "spec/features/login.feature", &original);

    // @step When I dispatch delete-scenario with feature='spec/features/login.feature' and scenario='Old scenario'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": "Old scenario"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file on disk equals the original text without the 'Old scenario' block
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert_eq!(
        after,
        format!("{BACKGROUND_HEADER}\n{FIRST_SCENARIO}\n{LAST_SCENARIO}")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher deletes a scenario whose name has quotes and an at sign
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_deletes_a_scenario_whose_name_has_quotes_and_an_at_sign() {
    // @step Given a project root tempdir with spec/features/login.feature containing scenarios "User can't login with @special chars" and 'Keep scenario'
    let tmp = TempDir::new().expect("tempdir");
    let name = "User can't login with @special chars";
    write_feature(
        tmp.path(),
        "spec/features/login.feature",
        &TWO_SCENARIO_FEATURE.replace("Old scenario", name),
    );

    // @step When I dispatch delete-scenario with feature='spec/features/login.feature' and scenario="User can't login with @special chars"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "spec/features/login.feature", "scenario": name}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher message equals "Successfully deleted scenario 'User can't login with @special chars' from login.feature"
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["message"].as_str(),
        Some("Successfully deleted scenario 'User can't login with @special chars' from login.feature")
    );

    // @step And the file on disk still contains 'Scenario: Keep scenario'
    let after = read_feature(tmp.path(), "spec/features/login.feature");
    assert!(after.contains("Scenario: Keep scenario"), "{after}");
    assert!(!after.contains(name), "{after}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects a feature file that is not valid Gherkin
// ─────────────────────────────────────────────────────────────────────────

/// A Scenario line with no Feature header above it does not parse.
const BROKEN_FEATURE: &str = "Scenario: Some scenario\n  Given a\n";

#[test]
fn scenario_dispatcher_rejects_a_feature_file_that_is_not_valid_gherkin() {
    // @step Given a project root tempdir with spec/features/broken.feature that does not parse as Gherkin
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/broken.feature", BROKEN_FEATURE);

    // @step When I dispatch delete-scenario with feature='broken' and scenario='Some scenario'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"feature": "broken", "scenario": "Some scenario"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the dispatcher error reason starts with 'Invalid Gherkin syntax:'
    let err = dispatcher_error(&result);
    let reason = err
        .strip_prefix("Invalid args for fspec command delete-scenario: ")
        .unwrap_or_else(|| panic!("not an invalid-args error: {err}"));
    assert!(reason.starts_with("Invalid Gherkin syntax:"), "{err}");

    // @step And spec/features/broken.feature is unchanged
    assert_eq!(
        read_feature(tmp.path(), "spec/features/broken.feature"),
        BROKEN_FEATURE
    );
}

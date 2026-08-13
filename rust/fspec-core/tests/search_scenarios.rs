#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::uninlined_format_args
)]
// Feature: spec/features/search-scenarios-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `search-scenarios`
// (RPC-297). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.
//
// Red phase: the dispatcher arm still calls the 1-arg stub which returns
// FspecCoreError::NotYetPorted, so these tests COMPILE and FAIL at runtime
// until the Phase C port lands.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────── helpers ─────────

fn req(project_root: &Path, args: Value) -> codelet_fspec_core::DispatchResult {
    dispatch_command(DispatchRequest {
        command: "search-scenarios".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

fn write_feature(root: &Path, name: &str, body: &str) {
    let path = root.join("spec/features").join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write feature");
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Literal query matches a scenario by name
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn literal_query_matches_a_scenario_by_name() {
    // @step Given a temp project root contains spec/features with a feature whose scenario is named "Login with valid credentials"
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "user-login.feature",
        "@AUTH-001\nFeature: User Authentication\n\n  Scenario: Login with valid credentials\n    Given I am on the login page\n",
    );

    // @step When I dispatch search-scenarios with query="Login"
    let result = req(tmp.path(), json!({"query": "Login"}));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let v: Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step And the scenarios array contains an entry with scenarioName "Login with valid credentials"
    let scenarios = v["scenarios"].as_array().expect("scenarios array");
    let entry = scenarios
        .iter()
        .find(|s| s["scenarioName"].as_str() == Some("Login with valid credentials"))
        .expect("entry with matching scenarioName");

    // @step And that entry carries its featureFilePath and workUnitId
    assert!(
        entry["featureFilePath"].as_str().is_some(),
        "entry must carry featureFilePath; got {entry:?}"
    );
    assert!(
        entry["workUnitId"].as_str().is_some(),
        "entry must carry workUnitId; got {entry:?}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Regex query matches multiple scenarios case-insensitively
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn regex_query_matches_multiple_scenarios_case_insensitively() {
    // @step Given a temp project root contains spec/features with scenarios named "Validate user" and "valid email"
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "validation.feature",
        "@VAL-001\nFeature: Validation\n\n  Scenario: Validate user\n    Given a user\n\n  Scenario: valid email\n    Given an email\n",
    );

    // @step When I dispatch search-scenarios with query="valid.*" and regex=true
    let result = req(tmp.path(), json!({"query": "valid.*", "regex": true}));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let v: Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step And the searchMode field equals 'regex'
    assert_eq!(v["searchMode"].as_str(), Some("regex"));

    // @step And the scenarios array has 2 elements
    assert_eq!(v["scenarios"].as_array().expect("scenarios array").len(), 2);
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: No match returns an empty scenarios array
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn no_match_returns_an_empty_scenarios_array() {
    // @step Given a temp project root contains spec/features with at least one feature file
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "user-login.feature",
        "@AUTH-001\nFeature: User Authentication\n\n  Scenario: Login with valid credentials\n    Given I am on the login page\n",
    );

    // @step When I dispatch search-scenarios with query="zzz-nonexistent-zzz"
    let result = req(tmp.path(), json!({"query": "zzz-nonexistent-zzz"}));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let v: Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step And the searchedFiles field is greater than 0
    assert!(
        v["searchedFiles"].as_u64().expect("searchedFiles") > 0,
        "searchedFiles must be > 0; got {v}"
    );

    // @step And the scenarios array is empty
    assert!(v["scenarios"]
        .as_array()
        .expect("scenarios array")
        .is_empty());
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Feature-name match returns all of that feature's scenarios
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn feature_name_match_returns_all_of_that_features_scenarios() {
    // @step Given a temp project root contains a feature named "User Authentication" with two scenarios
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "user-authentication.feature",
        "@AUTH-001\nFeature: User Authentication\n\n  Scenario: First step\n    Given a\n\n  Scenario: Second step\n    Given b\n",
    );

    // @step When I dispatch search-scenarios with query="Authentication"
    let result = req(tmp.path(), json!({"query": "Authentication"}));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let v: Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step And the scenarios array has 2 elements
    assert_eq!(v["scenarios"].as_array().expect("scenarios array").len(), 2);
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: workUnitId falls back to unknown when feature has no work-unit tag
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn work_unit_id_falls_back_to_unknown_when_feature_has_no_work_unit_tag() {
    // @step Given a temp project root contains an untagged feature with one scenario
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "untagged.feature",
        "Feature: Untagged Feature\n\n  Scenario: Lonely scenario\n    Given a\n",
    );

    // @step When I dispatch search-scenarios with query matching that scenario
    let result = req(tmp.path(), json!({"query": "Lonely scenario"}));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let v: Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step And the matching scenario's workUnitId equals 'unknown'
    let scenarios = v["scenarios"].as_array().expect("scenarios array");
    let entry = scenarios
        .iter()
        .find(|s| s["scenarioName"].as_str() == Some("Lonely scenario"))
        .expect("matching scenario");
    assert_eq!(entry["workUnitId"].as_str(), Some("unknown"));
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Invalid regex pattern surfaces a structured error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn invalid_regex_pattern_surfaces_a_structured_error() {
    // @step Given a temp project root contains spec/features with at least one feature file
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "user-login.feature",
        "@AUTH-001\nFeature: User Authentication\n\n  Scenario: Login with valid credentials\n    Given I am on the login page\n",
    );

    // @step When I dispatch search-scenarios with query="[" and regex=true
    let result = req(tmp.path(), json!({"query": "[", "regex": true}));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error field contains the substring 'regex'
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains("regex"),
        "error must mention regex; got {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Missing spec/features directory yields zero searched files
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn missing_spec_features_directory_yields_zero_searched_files() {
    // @step Given a temp project root with no spec/features directory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/features").exists());

    // @step When I dispatch search-scenarios with query="anything"
    let result = req(tmp.path(), json!({"query": "anything"}));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let v: Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step And the searchedFiles field equals 0
    assert_eq!(v["searchedFiles"].as_u64(), Some(0));

    // @step And the scenarios array is empty
    assert!(v["scenarios"]
        .as_array()
        .expect("scenarios array")
        .is_empty());
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: json format flag sets the format field to json
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn json_format_flag_sets_the_format_field_to_json() {
    // @step Given a temp project root contains spec/features with at least one feature file
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "user-login.feature",
        "@AUTH-001\nFeature: User Authentication\n\n  Scenario: Login with valid credentials\n    Given I am on the login page\n",
    );

    // @step When I dispatch search-scenarios with query="Login" and json=true
    let result = req(tmp.path(), json!({"query": "Login", "json": true}));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let v: Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step And the format field equals 'json'
    assert_eq!(v["format"].as_str(), Some("json"));
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Shared infrastructure module is registered for search-scenarios
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn shared_infrastructure_module_is_registered_for_search_scenarios() {
    // @step Given the rust/fspec-core crate is built
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "user-login.feature",
        "@AUTH-001\nFeature: User Authentication\n\n  Scenario: Login with valid credentials\n    Given I am on the login page\n",
    );

    // @step When I inspect rust/fspec-core/src/commands/search_scenarios.rs
    let result = req(tmp.path(), json!({"query": "Login"}));

    // @step Then the module no longer returns FspecCoreError::NotYetPorted
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        !err.contains("NotYetPorted")
            && !err.contains("not yet ported")
            && !err.contains("RPC-297"),
        "module must no longer return NotYetPorted; got error: {err:?}"
    );

    // @step And the dispatcher routes search-scenarios to the new run function
    assert!(
        result.success,
        "dispatcher must succeed when args are valid; got {result:?}"
    );
}

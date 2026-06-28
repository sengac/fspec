#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::uninlined_format_args
)]
// Feature: spec/features/unlink-coverage-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `unlink-coverage`
// (RPC-311). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.
//
// Red phase: the dispatcher arm still calls the 1-arg stub which returns
// FspecCoreError::NotYetPorted, so these tests COMPILE and FAIL at runtime
// until the Phase C port lands.
//
// Coverage-sidecar fixtures are seeded inline, pretty-printed and visually
// inspected (no duplicate keys).

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────── helpers ─────────

fn req(project_root: &Path, args: Value) -> codelet_fspec_core::DispatchResult {
    dispatch_command(DispatchRequest {
        command: "unlink-coverage".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

fn write_file(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

/// Read back the written sidecar for `user-login` and parse it as JSON.
fn read_sidecar(root: &Path) -> Value {
    let raw = fs::read_to_string(root.join("spec/features/user-login.feature.coverage"))
        .expect("read sidecar");
    serde_json::from_str(&raw).expect("sidecar is JSON")
}

/// Find scenario by name in a parsed sidecar.
fn scenario<'a>(sidecar: &'a Value, name: &str) -> &'a Value {
    sidecar["scenarios"]
        .as_array()
        .expect("scenarios array")
        .iter()
        .find(|s| s["name"].as_str() == Some(name))
        .expect("scenario present")
}

/// Sidecar where scenario "Login" has one test mapping for src/auth.test.ts
/// carrying one impl mapping for src/old.ts.
fn sidecar_login_with_test_and_impl() -> &'static str {
    r#"{
  "scenarios": [
    {
      "name": "Login",
      "testMappings": [
        {
          "file": "src/auth.test.ts",
          "lines": "1-10",
          "implMappings": [
            { "file": "src/old.ts", "lines": [1, 2, 3] }
          ]
        }
      ]
    }
  ],
  "stats": {
    "totalScenarios": 1,
    "coveredScenarios": 1,
    "coveragePercent": 100,
    "testFiles": ["src/auth.test.ts"],
    "implFiles": ["src/old.ts"],
    "totalLinesCovered": 13
  }
}"#
}

/// Sidecar where scenario "Login" exists but has no test mappings.
fn sidecar_login_no_mappings() -> &'static str {
    r#"{
  "scenarios": [
    {
      "name": "Login",
      "testMappings": []
    }
  ],
  "stats": {
    "totalScenarios": 1,
    "coveredScenarios": 0,
    "coveragePercent": 0,
    "testFiles": [],
    "implFiles": [],
    "totalLinesCovered": 0
  }
}"#
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: --all empties the scenario testMappings and recalculates stats
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn all_empties_the_scenario_test_mappings_and_recalculates_stats() {
    // @step Given a temp project root has a coverage sidecar where scenario "Login" has one test mapping with impl mappings
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        sidecar_login_with_test_and_impl(),
    );

    // @step When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and all=true
    let result = req(
        tmp.path(),
        json!({"featureName": "user-login", "scenario": "Login", "all": true}),
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let sidecar = read_sidecar(tmp.path());

    // @step And the scenario "Login" testMappings array is empty in the written sidecar
    assert!(
        scenario(&sidecar, "Login")["testMappings"]
            .as_array()
            .expect("testMappings array")
            .is_empty(),
        "Login testMappings must be empty; got {sidecar}"
    );

    // @step And the stats coveragePercent reflects the removed coverage
    assert_eq!(
        sidecar["stats"]["coveragePercent"].as_i64(),
        Some(0),
        "coveragePercent must drop to 0; got {sidecar}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: --test-file removes the whole test mapping including impl mappings
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn test_file_removes_the_whole_test_mapping_including_impl_mappings() {
    // @step Given a temp project root has a coverage sidecar where scenario "Login" has a test mapping for "src/auth.test.ts" with impl mappings
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        sidecar_login_with_test_and_impl(),
    );

    // @step When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and testFile="src/auth.test.ts"
    let result = req(
        tmp.path(),
        json!({"featureName": "user-login", "scenario": "Login", "testFile": "src/auth.test.ts"}),
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let sidecar = read_sidecar(tmp.path());

    // @step And the scenario "Login" has no test mapping for "src/auth.test.ts" in the written sidecar
    let has_mapping = scenario(&sidecar, "Login")["testMappings"]
        .as_array()
        .expect("testMappings array")
        .iter()
        .any(|tm| tm["file"].as_str() == Some("src/auth.test.ts"));
    assert!(!has_mapping, "test mapping must be removed; got {sidecar}");
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: --test-file with --impl-file removes only the impl mapping
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn test_file_with_impl_file_removes_only_the_impl_mapping() {
    // @step Given a temp project root has a coverage sidecar where scenario "Login" has a test mapping for "src/auth.test.ts" with an impl mapping for "src/old.ts"
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        sidecar_login_with_test_and_impl(),
    );

    // @step When I dispatch unlink-coverage for feature "user-login" with scenario="Login", testFile="src/auth.test.ts" and implFile="src/old.ts"
    let result = req(
        tmp.path(),
        json!({
            "featureName": "user-login",
            "scenario": "Login",
            "testFile": "src/auth.test.ts",
            "implFile": "src/old.ts"
        }),
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let sidecar = read_sidecar(tmp.path());

    // @step And the test mapping for "src/auth.test.ts" still exists in the written sidecar
    let tm = scenario(&sidecar, "Login")["testMappings"]
        .as_array()
        .expect("testMappings array")
        .iter()
        .find(|tm| tm["file"].as_str() == Some("src/auth.test.ts"))
        .expect("test mapping for src/auth.test.ts must remain");

    // @step And that test mapping no longer references "src/old.ts"
    let still_refs_old = tm["implMappings"]
        .as_array()
        .expect("implMappings array")
        .iter()
        .any(|im| im["file"].as_str() == Some("src/old.ts"));
    assert!(
        !still_refs_old,
        "impl mapping src/old.ts must be removed; got {sidecar}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Neither --all nor --test-file surfaces a validation error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn neither_all_nor_test_file_surfaces_a_validation_error() {
    // @step Given a temp project root has a coverage sidecar with scenario "Login"
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        sidecar_login_no_mappings(),
    );

    // @step When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and no test-file or all flag
    let result = req(
        tmp.path(),
        json!({"featureName": "user-login", "scenario": "Login"}),
    );

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error field contains the substring 'Must specify either --all or --test-file'
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Must specify either --all or --test-file"),
        "error must mention required flag; got {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: --impl-file without --test-file or --all surfaces the required-flag error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn impl_file_without_test_file_surfaces_a_validation_error() {
    // @step Given a temp project root has a coverage sidecar with scenario "Login"
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        sidecar_login_no_mappings(),
    );

    // @step When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and implFile="src/old.ts" and no test-file
    let result = req(
        tmp.path(),
        json!({"featureName": "user-login", "scenario": "Login", "implFile": "src/old.ts"}),
    );

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error field contains the substring 'Must specify either --all or --test-file'
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Must specify either --all or --test-file"),
        "error must mention required flag; got {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Missing coverage file surfaces a not-found error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn missing_coverage_file_surfaces_a_not_found_error() {
    // @step Given a temp project root has no coverage sidecar for feature "user-login"
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("spec/features")).expect("mkdir");

    // @step When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and all=true
    let result = req(
        tmp.path(),
        json!({"featureName": "user-login", "scenario": "Login", "all": true}),
    );

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error field contains the substring 'Coverage file not found'
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Coverage file not found"),
        "error must mention coverage file not found; got {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Unknown scenario surfaces a not-found error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn unknown_scenario_surfaces_a_not_found_error() {
    // @step Given a temp project root has a coverage sidecar with scenario "Login"
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        sidecar_login_no_mappings(),
    );

    // @step When I dispatch unlink-coverage for feature "user-login" with scenario="Logout" and all=true
    let result = req(
        tmp.path(),
        json!({"featureName": "user-login", "scenario": "Logout", "all": true}),
    );

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error field contains the substring 'Scenario not found'
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Scenario not found"),
        "error must mention scenario not found; got {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Unknown test file surfaces a not-found error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn unknown_test_file_surfaces_a_not_found_error() {
    // @step Given a temp project root has a coverage sidecar where scenario "Login" has a test mapping for "src/auth.test.ts"
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        sidecar_login_with_test_and_impl(),
    );

    // @step When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and testFile="src/missing.test.ts"
    let result = req(
        tmp.path(),
        json!({"featureName": "user-login", "scenario": "Login", "testFile": "src/missing.test.ts"}),
    );

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error field contains the substring 'Test file not found in scenario mappings'
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Test file not found in scenario mappings"),
        "error must mention test file not found; got {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Atomic write back preserves unknown fields in the sidecar
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn atomic_write_back_preserves_unknown_fields_in_the_sidecar() {
    // @step Given a temp project root has a coverage sidecar carrying an unknown top-level field alongside scenario "Login"
    let tmp = TempDir::new().expect("tempdir");
    let body = r#"{
  "customField": "keepme",
  "scenarios": [
    {
      "name": "Login",
      "testMappings": [
        {
          "file": "src/auth.test.ts",
          "lines": "1-10",
          "implMappings": [
            { "file": "src/old.ts", "lines": [1, 2, 3] }
          ]
        }
      ]
    }
  ],
  "stats": {
    "totalScenarios": 1,
    "coveredScenarios": 1,
    "coveragePercent": 100,
    "testFiles": ["src/auth.test.ts"],
    "implFiles": ["src/old.ts"],
    "totalLinesCovered": 13
  }
}"#;
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        body,
    );

    // @step When I dispatch unlink-coverage for feature "user-login" with scenario="Login" and all=true
    let result = req(
        tmp.path(),
        json!({"featureName": "user-login", "scenario": "Login", "all": true}),
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the written sidecar still contains the unknown top-level field
    let sidecar = read_sidecar(tmp.path());
    assert_eq!(
        sidecar["customField"].as_str(),
        Some("keepme"),
        "unknown top-level field must be preserved; got {sidecar}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Shared infrastructure module is registered for unlink-coverage
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn shared_infrastructure_module_is_registered_for_unlink_coverage() {
    // @step Given the codelet/fspec-core crate is built
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        sidecar_login_with_test_and_impl(),
    );

    // @step When I inspect codelet/fspec-core/src/commands/unlink_coverage.rs
    let result = req(
        tmp.path(),
        json!({"featureName": "user-login", "scenario": "Login", "all": true}),
    );

    // @step Then the module no longer returns FspecCoreError::NotYetPorted
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        !err.contains("NotYetPorted")
            && !err.contains("not yet ported")
            && !err.contains("RPC-311"),
        "module must no longer return NotYetPorted; got error: {err:?}"
    );

    // @step And the dispatcher routes unlink-coverage to the new run function
    assert!(
        result.success,
        "dispatcher must succeed when args are valid; got {result:?}"
    );
}

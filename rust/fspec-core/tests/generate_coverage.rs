#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::uninlined_format_args
)]
// Feature: spec/features/generate-coverage-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `generate-coverage`
// (RPC-231). Each scenario maps to exactly one #[test] function with
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
        command: "generate-coverage".to_string(),
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

/// A feature file with two scenarios: "Login" and "Logout".
fn feature_two_scenarios() -> &'static str {
    "Feature: User Login

  Scenario: Login
    Given I am on the login page
    When I enter valid credentials
    Then I see the dashboard

  Scenario: Logout
    Given I am logged in
    When I click logout
    Then I am on the login page
"
}

/// A feature file with a single scenario: "Login".
fn feature_one_scenario() -> &'static str {
    "Feature: User Login

  Scenario: Login
    Given I am on the login page
    When I enter valid credentials
    Then I see the dashboard
"
}

fn read_sidecar(root: &Path) -> Value {
    let raw = fs::read_to_string(root.join("spec/features/user-login.feature.coverage"))
        .expect("read sidecar");
    serde_json::from_str(&raw).expect("sidecar is JSON")
}

fn scenario<'a>(sidecar: &'a Value, name: &str) -> Option<&'a Value> {
    sidecar["scenarios"]
        .as_array()
        .expect("scenarios array")
        .iter()
        .find(|s| s["name"].as_str() == Some(name))
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Creates a sidecar for a feature file that lacks one
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn creates_a_sidecar_for_a_feature_file_that_lacks_one() {
    // @step Given a temp project root has a feature file "user-login.feature" with two scenarios and no coverage sidecar
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        feature_two_scenarios(),
    );

    // @step When I dispatch generate-coverage against that project root
    let result = req(tmp.path(), json!({}));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And a coverage sidecar "user-login.feature.coverage" is created with two scenario entries each having empty testMappings
    let sidecar = read_sidecar(tmp.path());
    let scenarios = sidecar["scenarios"].as_array().expect("scenarios array");
    assert_eq!(scenarios.len(), 2, "must have two scenarios; got {sidecar}");
    for s in scenarios {
        assert!(
            s["testMappings"]
                .as_array()
                .expect("testMappings array")
                .is_empty(),
            "each scenario must have empty testMappings; got {sidecar}"
        );
    }

    // @step And the created sidecar stats report totalScenarios 2, coveredScenarios 0 and coveragePercent 0
    assert_eq!(sidecar["stats"]["totalScenarios"].as_i64(), Some(2));
    assert_eq!(sidecar["stats"]["coveredScenarios"].as_i64(), Some(0));
    assert_eq!(sidecar["stats"]["coveragePercent"].as_i64(), Some(0));

    // @step And the rendered output contains the substring "Created 1"
    assert!(
        result.data.contains("Created 1"),
        "output must mention Created 1; got:\n{}",
        result.data
    );

    // @step And the rendered output contains the link-coverage system-reminder block
    assert!(
        result.data.contains("<system-reminder>")
            && result
                .data
                .contains("link-coverage POPULATES coverage files"),
        "output must include the link-coverage system-reminder; got:\n{}",
        result.data
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Skips a sidecar that is already in sync
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn skips_a_sidecar_that_is_already_in_sync() {
    // @step Given a temp project root has a feature file "user-login.feature" whose coverage sidecar already lists all its scenarios
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        feature_two_scenarios(),
    );
    let in_sync = r#"{
  "scenarios": [
    { "name": "Login", "testMappings": [] },
    { "name": "Logout", "testMappings": [] }
  ],
  "stats": {
    "totalScenarios": 2,
    "coveredScenarios": 0,
    "coveragePercent": 0,
    "testFiles": [],
    "implFiles": [],
    "totalLinesCovered": 0
  }
}"#;
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        in_sync,
    );

    // @step When I dispatch generate-coverage against that project root
    let result = req(tmp.path(), json!({}));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And the rendered output contains the substring "Skipped 1"
    assert!(
        result.data.contains("Skipped 1"),
        "output must mention Skipped 1; got:\n{}",
        result.data
    );

    // @step And the existing sidecar is left byte-for-byte unchanged
    let after = fs::read_to_string(tmp.path().join("spec/features/user-login.feature.coverage"))
        .expect("read sidecar");
    assert_eq!(after, in_sync, "in-sync sidecar must not be rewritten");
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Updates a sidecar when scenarios were added and removed
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn updates_a_sidecar_when_scenarios_were_added_and_removed() {
    // @step Given a temp project root has a feature file "user-login.feature" with one new scenario absent from its sidecar and one stale scenario only in the sidecar
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        feature_two_scenarios(),
    );
    // Sidecar contains "Login" (with a test mapping) and stale "OldStale";
    // feature contains "Login" + new "Logout".
    let stale = r#"{
  "scenarios": [
    {
      "name": "Login",
      "testMappings": [
        { "file": "src/auth.test.ts", "lines": "1-10", "implMappings": [] }
      ]
    },
    { "name": "OldStale", "testMappings": [] }
  ],
  "stats": {
    "totalScenarios": 2,
    "coveredScenarios": 1,
    "coveragePercent": 50,
    "testFiles": ["src/auth.test.ts"],
    "implFiles": [],
    "totalLinesCovered": 10
  }
}"#;
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        stale,
    );

    // @step When I dispatch generate-coverage against that project root
    let result = req(tmp.path(), json!({}));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");
    let sidecar = read_sidecar(tmp.path());

    // @step And the updated sidecar adds the new scenario with empty testMappings and drops the stale scenario
    let logout = scenario(&sidecar, "Logout").expect("Logout added");
    assert!(
        logout["testMappings"].as_array().expect("array").is_empty(),
        "new scenario must have empty testMappings; got {sidecar}"
    );
    assert!(
        scenario(&sidecar, "OldStale").is_none(),
        "stale scenario must be dropped; got {sidecar}"
    );

    // @step And the updated sidecar preserves the existing test mappings of unchanged scenarios
    let login = scenario(&sidecar, "Login").expect("Login kept");
    let has_mapping = login["testMappings"]
        .as_array()
        .expect("array")
        .iter()
        .any(|tm| tm["file"].as_str() == Some("src/auth.test.ts"));
    assert!(
        has_mapping,
        "Login's test mapping must be preserved; got {sidecar}"
    );

    // @step And the rendered output contains the substring "Updated 1"
    assert!(
        result.data.contains("Updated 1"),
        "output must mention Updated 1; got:\n{}",
        result.data
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Recreates a sidecar that contains invalid JSON
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn recreates_a_sidecar_that_contains_invalid_json() {
    // @step Given a temp project root has a feature file "user-login.feature" with a coverage sidecar whose contents are not valid JSON
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        feature_two_scenarios(),
    );
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        "this is not valid json {{{",
    );

    // @step When I dispatch generate-coverage against that project root
    let result = req(tmp.path(), json!({}));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And the sidecar is rewritten as valid JSON with one entry per scenario
    let sidecar = read_sidecar(tmp.path());
    let scenarios = sidecar["scenarios"].as_array().expect("scenarios array");
    assert_eq!(
        scenarios.len(),
        2,
        "must have one entry per scenario; got {sidecar}"
    );

    // @step And the rendered output contains the substring "Recreated 1"
    assert!(
        result.data.contains("Recreated 1"),
        "output must mention Recreated 1; got:\n{}",
        result.data
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Dry-run reports would-create files without writing
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn dry_run_reports_would_create_files_without_writing() {
    // @step Given a temp project root has a feature file "user-login.feature" and no coverage sidecar
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        feature_one_scenario(),
    );

    // @step When I dispatch generate-coverage with dryRun true against that project root
    let result = req(tmp.path(), json!({"dryRun": true}));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And no coverage sidecar is written to disk
    assert!(
        !tmp.path()
            .join("spec/features/user-login.feature.coverage")
            .exists(),
        "dry-run must not write a sidecar"
    );

    // @step And the rendered output contains the substring "Would create 1 coverage files (DRY RUN)"
    assert!(
        result
            .data
            .contains("Would create 1 coverage files (DRY RUN)"),
        "output must mention would-create count; got:\n{}",
        result.data
    );

    // @step And the rendered output lists "user-login.feature.coverage"
    assert!(
        result.data.contains("user-login.feature.coverage"),
        "output must list the coverage file name; got:\n{}",
        result.data
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Missing features directory surfaces an error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn missing_features_directory_surfaces_an_error() {
    // @step Given a temp project root has no spec/features directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch generate-coverage against that project root
    let result = req(tmp.path(), json!({}));

    // @step Then the dispatcher returns an error whose message contains "Failed to read features directory"
    assert!(!result.success, "expected failure; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Failed to read features directory"),
        "error must mention failed-to-read-features-dir; got {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// BUG-301: global dry-run parity, unknown fields, mapped stale scenarios
// ═════════════════════════════════════════════════════════════════════════

const DRIFTED_SIDECAR: &str = r#"{
  "scenarios": [
    { "name": "Login", "testMappings": [] },
    { "name": "OldStale", "testMappings": [] }
  ],
  "stats": { "totalScenarios": 2, "coveredScenarios": 0, "coveragePercent": 0 }
}"#;

#[test]
fn dry_run_reports_the_updates_a_real_run_would_make() {
    // @step Given a temp project root has a feature file "user-login.feature" with one new scenario absent from its sidecar and one stale scenario only in the sidecar
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        feature_two_scenarios(),
    );
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        DRIFTED_SIDECAR,
    );

    // @step When I dispatch generate-coverage with dryRun true against that project root
    let result = req(tmp.path(), json!({ "dryRun": true }));
    assert!(result.success, "expected success; got {result:?}");

    // @step Then the rendered output contains the substring "Would update 1 coverage files"
    assert!(
        result.data.contains("Would update 1 coverage files"),
        "{}",
        result.data
    );

    // @step And the rendered output names the added scenario "Logout" and the removed scenario "OldStale"
    assert!(result.data.contains("+ Logout"), "{}", result.data);
    assert!(result.data.contains("- OldStale"), "{}", result.data);

    // @step And the existing sidecar is left byte-for-byte unchanged
    let after = fs::read_to_string(tmp.path().join("spec/features/user-login.feature.coverage"))
        .expect("sidecar");
    assert_eq!(after, DRIFTED_SIDECAR);
}

#[test]
fn update_preserves_unknown_top_level_sidecar_fields() {
    // @step Given a temp project root has a feature file "user-login.feature" whose sidecar lacks one scenario and carries an extra top-level field
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        feature_two_scenarios(),
    );
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        r#"{"scenarios":[{"name":"Login","testMappings":[]}],"stats":{},"lineage":{"base":"626eee1e"}}"#,
    );

    // @step When I dispatch generate-coverage against that project root
    let result = req(tmp.path(), json!({}));
    assert!(result.success, "expected success; got {result:?}");

    // @step Then the updated sidecar still contains the extra top-level field
    let sidecar = read_sidecar(tmp.path());
    assert_eq!(
        sidecar["lineage"]["base"].as_str(),
        Some("626eee1e"),
        "{sidecar}"
    );
    assert!(scenario(&sidecar, "Logout").is_some(), "{sidecar}");
}

#[test]
fn update_keeps_a_stale_scenario_that_still_has_test_mappings() {
    // @step Given a temp project root has a feature file "user-login.feature" whose sidecar has a stale scenario with a test mapping
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        feature_two_scenarios(),
    );
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        r#"{"scenarios":[
            {"name":"Login","testMappings":[]},
            {"name":"OldMapped","testMappings":[{"file":"src/old.test.ts","lines":"1-9","implMappings":[]}]}
        ],"stats":{}}"#,
    );

    // @step When I dispatch generate-coverage against that project root
    let result = req(tmp.path(), json!({}));
    assert!(result.success, "expected success; got {result:?}");

    // @step Then the updated sidecar still contains the stale scenario with its test mapping
    let sidecar = read_sidecar(tmp.path());
    let kept = scenario(&sidecar, "OldMapped").expect("mapped stale scenario kept");
    assert_eq!(
        kept["testMappings"][0]["file"].as_str(),
        Some("src/old.test.ts")
    );
    assert!(scenario(&sidecar, "Logout").is_some(), "{sidecar}");

    // @step And the rendered output reports the blocked removal of "OldMapped"
    assert!(result.data.contains("OldMapped"), "{}", result.data);
    assert!(result.data.contains("--feature-name"), "{}", result.data);
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Created sidecar scenario names keep the exact case of the feature file
// (moved from auto-create-coverage-files, COV-072)
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn created_sidecar_scenario_names_keep_the_exact_case_of_the_feature_file() {
    // @step Given a temp project root has a feature file "user-login.feature" with the single scenario "Login With Valid Credentials" and no coverage sidecar
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        "Feature: User Login

  Scenario: Login With Valid Credentials
    Given I am on the login page
    When I enter valid credentials
    Then I see the dashboard
",
    );

    // @step When I dispatch generate-coverage against that project root
    let result = req(tmp.path(), json!({}));
    assert!(result.success, "expected success; got {result:?}");

    // @step Then the created sidecar has one scenario entry named exactly "Login With Valid Credentials"
    let sidecar = read_sidecar(tmp.path());
    let names: Vec<&str> = sidecar["scenarios"]
        .as_array()
        .expect("scenarios array")
        .iter()
        .filter_map(|s| s["name"].as_str())
        .collect();
    assert_eq!(names, vec!["Login With Valid Credentials"], "{sidecar}");

    // @step And the sidecar has no scenario entry named "login-with-valid-credentials" or "Login with valid credentials"
    assert!(
        scenario(&sidecar, "login-with-valid-credentials").is_none(),
        "{sidecar}"
    );
    assert!(
        scenario(&sidecar, "Login with valid credentials").is_none(),
        "{sidecar}"
    );
}

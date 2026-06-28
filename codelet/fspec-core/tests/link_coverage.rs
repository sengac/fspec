#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::uninlined_format_args
)]
// Feature: spec/features/link-coverage-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `link-coverage`
// (RPC-240). Each scenario maps to exactly one #[test] function with
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
        command: "link-coverage".to_string(),
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

fn read_sidecar(root: &Path) -> Value {
    let raw = fs::read_to_string(root.join("spec/features/user-login.feature.coverage"))
        .expect("read sidecar");
    serde_json::from_str(&raw).expect("sidecar is JSON")
}

fn scenario<'a>(sidecar: &'a Value, name: &str) -> &'a Value {
    sidecar["scenarios"]
        .as_array()
        .expect("scenarios array")
        .iter()
        .find(|s| s["name"].as_str() == Some(name))
        .expect("scenario present")
}

/// Feature file (story work unit) with one scenario "Login".
const FEATURE_STORY: &str = "@AUTH-001
Feature: User Login

  Scenario: Login
    Given I am on the login page
    When I enter valid credentials
    Then I see the dashboard
";

/// A test file whose @step comments match all of FEATURE_STORY's Login steps.
const TEST_MATCHING: &str = "// @step Given I am on the login page
// @step When I enter valid credentials
// @step Then I see the dashboard
test('login', () => {});
";

/// A test file missing the final @step comment.
const TEST_MISSING_STEP: &str = "// @step Given I am on the login page
// @step When I enter valid credentials
test('login', () => {});
";

/// Empty sidecar listing scenario "Login" with no mappings.
const SIDECAR_LOGIN_EMPTY: &str = r#"{
  "scenarios": [
    { "name": "Login", "testMappings": [] }
  ],
  "stats": {
    "totalScenarios": 1,
    "coveredScenarios": 0,
    "coveragePercent": 0,
    "testFiles": [],
    "implFiles": [],
    "totalLinesCovered": 0
  }
}"#;

/// Seed a story workspace: feature + matching test + empty sidecar.
fn seed_story(root: &Path) {
    write_file(root, "spec/features/user-login.feature", FEATURE_STORY);
    write_file(
        root,
        "spec/features/user-login.feature.coverage",
        SIDECAR_LOGIN_EMPTY,
    );
    write_file(root, "src/auth.test.ts", TEST_MATCHING);
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: test-only mode appends a test mapping and recalculates stats
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn test_only_mode_appends_a_test_mapping_and_recalculates_stats() {
    // @step Given a temp project root has a feature file and matching coverage sidecar with scenario "Login", and a test file containing @step comments matching the scenario steps
    let tmp = TempDir::new().expect("tempdir");
    seed_story(tmp.path());

    // @step When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile and testLines
    let result = req(
        tmp.path(),
        json!({
            "featureName": "user-login",
            "scenario": "Login",
            "testFile": "src/auth.test.ts",
            "testLines": "45-62"
        }),
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let sidecar = read_sidecar(tmp.path());

    // @step And the scenario "Login" gains a test mapping referencing that test file and line range
    let tm = scenario(&sidecar, "Login")["testMappings"]
        .as_array()
        .expect("testMappings array")
        .iter()
        .find(|tm| tm["file"].as_str() == Some("src/auth.test.ts"))
        .expect("test mapping present");
    assert_eq!(tm["lines"].as_str(), Some("45-62"));

    // @step And the stats coveredScenarios and coveragePercent increase accordingly
    assert_eq!(sidecar["stats"]["coveredScenarios"].as_i64(), Some(1));
    assert_eq!(sidecar["stats"]["coveragePercent"].as_i64(), Some(100));

    // @step And the result message contains "Linked test mapping"
    assert!(
        result.data.contains("Linked test mapping"),
        "message must mention Linked test mapping; got:\n{}",
        result.data
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: impl-only mode adds an implementation mapping to an existing test mapping
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn impl_only_mode_adds_an_implementation_mapping_to_an_existing_test_mapping() {
    // @step Given a temp project root has a coverage sidecar where scenario "Login" already has a test mapping for the test file
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        FEATURE_STORY,
    );
    write_file(tmp.path(), "src/auth.test.ts", TEST_MATCHING);
    write_file(
        tmp.path(),
        "src/login.ts",
        "export const login = () => {};\n",
    );
    let sidecar = r#"{
  "scenarios": [
    {
      "name": "Login",
      "testMappings": [
        { "file": "src/auth.test.ts", "lines": "45-62", "implMappings": [] }
      ]
    }
  ],
  "stats": {
    "totalScenarios": 1,
    "coveredScenarios": 1,
    "coveragePercent": 100,
    "testFiles": ["src/auth.test.ts"],
    "implFiles": [],
    "totalLinesCovered": 18
  }
}"#;
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        sidecar,
    );

    // @step When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile, implFile and implLines "10-12"
    let result = req(
        tmp.path(),
        json!({
            "featureName": "user-login",
            "scenario": "Login",
            "testFile": "src/auth.test.ts",
            "implFile": "src/login.ts",
            "implLines": "10-12"
        }),
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let sidecar = read_sidecar(tmp.path());

    // @step And the test mapping gains an implementation mapping with lines [10, 11, 12]
    let tm = scenario(&sidecar, "Login")["testMappings"]
        .as_array()
        .expect("testMappings array")
        .iter()
        .find(|tm| tm["file"].as_str() == Some("src/auth.test.ts"))
        .expect("test mapping present");
    let im = tm["implMappings"]
        .as_array()
        .expect("implMappings array")
        .iter()
        .find(|im| im["file"].as_str() == Some("src/login.ts"))
        .expect("impl mapping present");
    let lines: Vec<i64> = im["lines"]
        .as_array()
        .expect("lines array")
        .iter()
        .map(|n| n.as_i64().expect("line number"))
        .collect();
    assert_eq!(
        lines,
        vec![10, 11, 12],
        "impl lines must expand the range; got {sidecar}"
    );

    // @step And the result message contains "implementation mapping"
    assert!(
        result.data.contains("implementation mapping"),
        "message must mention implementation mapping; got:\n{}",
        result.data
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: both mode appends a test mapping carrying its implementation mapping
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn both_mode_appends_a_test_mapping_carrying_its_implementation_mapping() {
    // @step Given a temp project root has a feature file and matching coverage sidecar with scenario "Login", and a test file with matching @step comments
    let tmp = TempDir::new().expect("tempdir");
    seed_story(tmp.path());
    write_file(
        tmp.path(),
        "src/login.ts",
        "export const login = () => {};\n",
    );

    // @step When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile, testLines, implFile and implLines
    let result = req(
        tmp.path(),
        json!({
            "featureName": "user-login",
            "scenario": "Login",
            "testFile": "src/auth.test.ts",
            "testLines": "45-62",
            "implFile": "src/login.ts",
            "implLines": "10-24"
        }),
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let sidecar = read_sidecar(tmp.path());

    // @step And the scenario gains a test mapping whose implMappings includes the implementation file
    let tm = scenario(&sidecar, "Login")["testMappings"]
        .as_array()
        .expect("testMappings array")
        .iter()
        .find(|tm| tm["file"].as_str() == Some("src/auth.test.ts"))
        .expect("test mapping present");
    let has_impl = tm["implMappings"]
        .as_array()
        .expect("implMappings array")
        .iter()
        .any(|im| im["file"].as_str() == Some("src/login.ts"));
    assert!(
        has_impl,
        "implMappings must include src/login.ts; got {sidecar}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: impl-file without test-file surfaces a flag-combination error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn impl_file_without_test_file_surfaces_a_flag_combination_error() {
    // @step Given a temp project root has a coverage sidecar with scenario "Login"
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        SIDECAR_LOGIN_EMPTY,
    );

    // @step When I dispatch link-coverage for feature "user-login" with scenario="Login" and implFile only
    let result = req(
        tmp.path(),
        json!({"featureName": "user-login", "scenario": "Login", "implFile": "src/login.ts"}),
    );

    // @step Then the dispatcher returns an error whose message contains "--test-file is required when adding implementation mappings"
    assert!(!result.success, "expected failure; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("--test-file is required when adding implementation mappings"),
        "error must mention test-file requirement; got {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: test-file without test-lines surfaces a flag-combination error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn test_file_without_test_lines_surfaces_a_flag_combination_error() {
    // @step Given a temp project root has a coverage sidecar with scenario "Login"
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        SIDECAR_LOGIN_EMPTY,
    );

    // @step When I dispatch link-coverage for feature "user-login" with scenario="Login" and testFile only
    let result = req(
        tmp.path(),
        json!({"featureName": "user-login", "scenario": "Login", "testFile": "src/auth.test.ts"}),
    );

    // @step Then the dispatcher returns an error whose message contains "--test-lines is required when linking test file"
    assert!(!result.success, "expected failure; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("--test-lines is required when linking test file"),
        "error must mention test-lines requirement; got {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: A missing test file without skip-validation errors
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn a_missing_test_file_without_skip_validation_errors() {
    // @step Given a temp project root has a coverage sidecar with scenario "Login" and no test file on disk
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        SIDECAR_LOGIN_EMPTY,
    );

    // @step When I dispatch link-coverage for feature "user-login" with scenario="Login", a non-existent testFile and testLines
    let result = req(
        tmp.path(),
        json!({
            "featureName": "user-login",
            "scenario": "Login",
            "testFile": "src/missing.test.ts",
            "testLines": "1-2"
        }),
    );

    // @step Then the dispatcher returns an error whose message contains "File not found"
    assert!(!result.success, "expected failure; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("File not found"),
        "error must mention File not found; got {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: skip-validation downgrades a missing file to a warning
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn skip_validation_downgrades_a_missing_file_to_a_warning() {
    // @step Given a temp project root has a feature file and coverage sidecar with scenario "Login" tagged as a task work unit, and no test file on disk
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        "@TASK-001\nFeature: User Login\n\n  Scenario: Login\n    Given I am on the login page\n    When I enter valid credentials\n    Then I see the dashboard\n",
    );
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        SIDECAR_LOGIN_EMPTY,
    );
    write_file(
        tmp.path(),
        "spec/work-units.json",
        r#"{ "workUnits": { "TASK-001": { "type": "task" } } }"#,
    );

    // @step When I dispatch link-coverage for feature "user-login" with scenario="Login", a non-existent testFile, testLines, skipValidation true and skipStepValidation true
    let result = req(
        tmp.path(),
        json!({
            "featureName": "user-login",
            "scenario": "Login",
            "testFile": "src/missing.test.ts",
            "testLines": "1-2",
            "skipValidation": true,
            "skipStepValidation": true
        }),
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the result warnings contain "validation skipped"
    assert!(
        result.data.contains("validation skipped"),
        "output must contain a 'validation skipped' warning; got:\n{}",
        result.data
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: A missing coverage sidecar errors with a generate-coverage suggestion
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn a_missing_coverage_sidecar_errors_with_a_generate_coverage_suggestion() {
    // @step Given a temp project root has a feature file with scenarios but no coverage sidecar
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        FEATURE_STORY,
    );
    write_file(tmp.path(), "src/auth.test.ts", TEST_MATCHING);

    // @step When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile and testLines
    let result = req(
        tmp.path(),
        json!({
            "featureName": "user-login",
            "scenario": "Login",
            "testFile": "src/auth.test.ts",
            "testLines": "45-62"
        }),
    );

    // @step Then the dispatcher returns an error whose message contains "Coverage file not found"
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        err.contains("Coverage file not found"),
        "error must mention coverage file not found; got {:?}",
        result.error
    );

    // @step And the error message suggests running fspec generate-coverage
    assert!(
        err.contains("fspec generate-coverage"),
        "error must suggest generate-coverage; got {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: A scenario absent from the sidecar errors with available scenarios listed
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn a_scenario_absent_from_the_sidecar_errors_with_available_scenarios_listed() {
    // @step Given a temp project root has a coverage sidecar that does not contain scenario "Nope"
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        FEATURE_STORY,
    );
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        SIDECAR_LOGIN_EMPTY,
    );
    write_file(tmp.path(), "src/auth.test.ts", TEST_MATCHING);

    // @step When I dispatch link-coverage for feature "user-login" with scenario="Nope", testFile and testLines
    let result = req(
        tmp.path(),
        json!({
            "featureName": "user-login",
            "scenario": "Nope",
            "testFile": "src/auth.test.ts",
            "testLines": "45-62"
        }),
    );

    // @step Then the dispatcher returns an error whose message contains "Scenario not found"
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        err.contains("Scenario not found"),
        "error must mention scenario not found; got {:?}",
        result.error
    );

    // @step And the error message lists the available scenarios
    assert!(
        err.contains("Login"),
        "error must list available scenarios; got {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Step validation fails when a required step comment is missing
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn step_validation_fails_when_a_required_step_comment_is_missing() {
    // @step Given a temp project root has a story feature file whose scenario "Login" has steps, a coverage sidecar, and a test file missing one required @step comment
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        FEATURE_STORY,
    );
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        SIDECAR_LOGIN_EMPTY,
    );
    write_file(tmp.path(), "src/auth.test.ts", TEST_MISSING_STEP);

    // @step When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile and testLines
    let result = req(
        tmp.path(),
        json!({
            "featureName": "user-login",
            "scenario": "Login",
            "testFile": "src/auth.test.ts",
            "testLines": "1-3"
        }),
    );

    // @step Then the dispatcher returns an error whose message contains "STEP VALIDATION FAILED"
    assert!(!result.success, "expected failure; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("STEP VALIDATION FAILED"),
        "error must mention STEP VALIDATION FAILED; got {:?}",
        result.error
    );

    // @step And the sidecar is not modified
    let after = fs::read_to_string(tmp.path().join("spec/features/user-login.feature.coverage"))
        .expect("read sidecar");
    assert_eq!(
        after, SIDECAR_LOGIN_EMPTY,
        "sidecar must be unchanged on validation failure"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: skip-step-validation is rejected for a story work unit
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn skip_step_validation_is_rejected_for_a_story_work_unit() {
    // @step Given a temp project root has a story feature file for scenario "Login", a coverage sidecar, and a test file missing required @step comments
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        FEATURE_STORY,
    );
    write_file(
        tmp.path(),
        "spec/features/user-login.feature.coverage",
        SIDECAR_LOGIN_EMPTY,
    );
    write_file(tmp.path(), "src/auth.test.ts", TEST_MISSING_STEP);
    write_file(
        tmp.path(),
        "spec/work-units.json",
        r#"{ "workUnits": { "AUTH-001": { "type": "story" } } }"#,
    );

    // @step When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile, testLines and skipStepValidation true
    let result = req(
        tmp.path(),
        json!({
            "featureName": "user-login",
            "scenario": "Login",
            "testFile": "src/auth.test.ts",
            "testLines": "1-3",
            "skipStepValidation": true
        }),
    );

    // @step Then the dispatcher returns an error whose message contains "STEP VALIDATION ENFORCEMENT VIOLATION"
    assert!(!result.success, "expected failure; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("STEP VALIDATION ENFORCEMENT VIOLATION"),
        "error must mention enforcement violation; got {:?}",
        result.error
    );

    // @step And the sidecar is not modified
    let after = fs::read_to_string(tmp.path().join("spec/features/user-login.feature.coverage"))
        .expect("read sidecar");
    assert_eq!(
        after, SIDECAR_LOGIN_EMPTY,
        "sidecar must be unchanged when skip is rejected"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Written sidecar preserves unknown top-level fields and is atomic 2-space JSON
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn written_sidecar_preserves_unknown_top_level_fields_and_is_atomic_2_space_json() {
    // @step Given a temp project root has a coverage sidecar carrying an unknown top-level field and scenario "Login" with a matching test file
    let tmp = TempDir::new().expect("tempdir");
    write_file(
        tmp.path(),
        "spec/features/user-login.feature",
        FEATURE_STORY,
    );
    write_file(tmp.path(), "src/auth.test.ts", TEST_MATCHING);
    let body = r#"{
  "customField": "keepme",
  "scenarios": [
    { "name": "Login", "testMappings": [] }
  ],
  "stats": {
    "totalScenarios": 1,
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
        body,
    );

    // @step When I dispatch link-coverage for feature "user-login" with scenario="Login", testFile and testLines
    let result = req(
        tmp.path(),
        json!({
            "featureName": "user-login",
            "scenario": "Login",
            "testFile": "src/auth.test.ts",
            "testLines": "45-62"
        }),
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the rewritten sidecar still contains the unknown top-level field
    let sidecar = read_sidecar(tmp.path());
    assert_eq!(
        sidecar["customField"].as_str(),
        Some("keepme"),
        "unknown top-level field must be preserved; got {sidecar}"
    );
}

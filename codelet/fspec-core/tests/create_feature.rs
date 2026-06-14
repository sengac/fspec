#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/create-feature-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `create-feature` (RPC-212).
// Each scenario maps to one #[test] fn with @step comments mirroring the
// Gherkin steps verbatim. At PHASE B time the command is still a stub, so the
// dispatcher returns success=false with a NotYetPorted error — these tests are
// expected to FAIL until PHASE C lands the implementation.

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
        command: "create-feature".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn empty_spec(project_root: &Path) {
    fs::create_dir_all(project_root.join("spec").join("features")).expect("mkdir spec/features");
}

fn read_file(project_root: &Path, rel: &str) -> String {
    fs::read_to_string(project_root.join(rel)).expect("read file")
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Creates feature file and coverage sidecar from a capability name
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_creates_feature_file_and_coverage_sidecar_from_capability_name() {
    // @step Given a project root tempdir with an empty spec directory
    let tmp = TempDir::new().expect("tempdir");
    empty_spec(tmp.path());

    // @step When I dispatch create-feature with name='User Authentication'
    let result = dispatch_command(req(tmp.path(), json!({"name": "User Authentication"})));

    // @step Then the dispatcher returns a filePath ending with 'spec/features/user-authentication.feature'
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let file_path = data["filePath"].as_str().unwrap_or("");
    assert!(
        file_path.ends_with("spec/features/user-authentication.feature"),
        "unexpected filePath: {file_path}"
    );

    // @step And the file spec/features/user-authentication.feature exists on disk
    assert!(tmp
        .path()
        .join("spec/features/user-authentication.feature")
        .exists());

    // @step And the sidecar spec/features/user-authentication.feature.coverage exists on disk
    assert!(tmp
        .path()
        .join("spec/features/user-authentication.feature.coverage")
        .exists());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Generated content matches the canonical template verbatim
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_generated_content_matches_canonical_template_verbatim() {
    // @step Given a project root tempdir with an empty spec directory
    let tmp = TempDir::new().expect("tempdir");
    empty_spec(tmp.path());

    // @step When I dispatch create-feature with name='User Authentication'
    let result = dispatch_command(req(tmp.path(), json!({"name": "User Authentication"})));
    assert!(result.success, "expected success=true, got {result:?}");

    let content = read_file(tmp.path(), "spec/features/user-authentication.feature");

    // @step Then the written file begins with the line '@critical @component @feature-group'
    assert!(
        content.starts_with("@critical @component @feature-group"),
        "unexpected start:\n{content}"
    );

    // @step And the written file contains the line 'Feature: User Authentication'
    assert!(content.contains("Feature: User Authentication"), "missing Feature line:\n{content}");

    // @step And the written file contains the placeholder steps '[precondition]', '[action]', and '[expected outcome]'
    assert!(content.contains("[precondition]"), "missing [precondition]:\n{content}");
    assert!(content.contains("[action]"), "missing [action]:\n{content}");
    assert!(content.contains("[expected outcome]"), "missing [expected outcome]:\n{content}");

    // @step And the written file ends with a trailing newline
    assert!(content.ends_with('\n'), "must end with trailing newline");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Coverage sidecar carries one empty scenario mapping with zeroed stats
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_coverage_sidecar_carries_one_empty_scenario_mapping_with_zeroed_stats() {
    // @step Given a project root tempdir with an empty spec directory
    let tmp = TempDir::new().expect("tempdir");
    empty_spec(tmp.path());

    // @step When I dispatch create-feature with name='User Authentication'
    let result = dispatch_command(req(tmp.path(), json!({"name": "User Authentication"})));
    assert!(result.success, "expected success=true, got {result:?}");

    let cov_raw = read_file(tmp.path(), "spec/features/user-authentication.feature.coverage");
    let cov: Value = serde_json::from_str(&cov_raw).expect("parse coverage json");

    // @step Then the coverage sidecar parses to one scenario named '[Scenario name]' with empty testMappings
    let scenarios = cov["scenarios"].as_array().expect("scenarios array");
    assert_eq!(scenarios.len(), 1, "expected exactly one scenario");
    assert_eq!(scenarios[0]["name"].as_str(), Some("[Scenario name]"));
    assert_eq!(
        scenarios[0]["testMappings"].as_array().map(Vec::len),
        Some(0),
        "testMappings must be empty"
    );

    // @step And the coverage sidecar stats report totalScenarios=1, coveredScenarios=0, and coveragePercent=0
    assert_eq!(cov["stats"]["totalScenarios"].as_u64(), Some(1));
    assert_eq!(cov["stats"]["coveredScenarios"].as_u64(), Some(0));
    assert_eq!(cov["stats"]["coveragePercent"].as_i64(), Some(0));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Prefill detection reports placeholders in the template
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_prefill_detection_reports_placeholders_in_the_template() {
    // @step Given a project root tempdir with an empty spec directory
    let tmp = TempDir::new().expect("tempdir");
    empty_spec(tmp.path());

    // @step When I dispatch create-feature with name='User Authentication'
    let result = dispatch_command(req(tmp.path(), json!({"name": "User Authentication"})));
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");

    // @step Then the dispatcher response reports prefillDetection.hasPrefill = true
    assert_eq!(
        data["prefillDetection"]["hasPrefill"].as_bool(),
        Some(true),
        "hasPrefill must be true"
    );

    // @step And the dispatcher response carries a prefill systemReminder string
    assert!(
        data["prefillDetection"]["systemReminder"].as_str().is_some(),
        "prefill systemReminder string expected"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Capability-style name emits no file-naming reminder
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_capability_style_name_emits_no_file_naming_reminder() {
    // @step Given a project root tempdir with an empty spec directory
    let tmp = TempDir::new().expect("tempdir");
    empty_spec(tmp.path());

    // @step When I dispatch create-feature with name='User Authentication'
    let result = dispatch_command(req(tmp.path(), json!({"name": "User Authentication"})));
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");

    // @step Then the dispatcher response has no fileNamingReminder field
    assert!(
        data.get("fileNamingReminder").map(serde_json::Value::is_null).unwrap_or(true),
        "fileNamingReminder must be absent/null for capability names; got {data}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Task-style name emits a file-naming anti-pattern reminder
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_task_style_name_emits_file_naming_anti_pattern_reminder() {
    // @step Given a project root tempdir with an empty spec directory
    let tmp = TempDir::new().expect("tempdir");
    empty_spec(tmp.path());

    // @step When I dispatch create-feature with name='Implement Login'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Implement Login"})));
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");

    // @step Then the dispatcher returns a filePath ending with 'spec/features/implement-login.feature'
    let file_path = data["filePath"].as_str().unwrap_or("");
    assert!(
        file_path.ends_with("spec/features/implement-login.feature"),
        "unexpected filePath: {file_path}"
    );

    // @step And the dispatcher response carries a fileNamingReminder string mentioning capabilities
    let reminder = data["fileNamingReminder"].as_str().unwrap_or("");
    assert!(
        reminder.to_lowercase().contains("capabilit"),
        "fileNamingReminder should mention capabilities; got: {reminder}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Creating an existing feature file fails without overwriting
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_creating_existing_feature_file_fails_without_overwriting() {
    // @step Given a project root tempdir whose spec/features/user-authentication.feature already exists with body 'KEEP ME\n'
    let tmp = TempDir::new().expect("tempdir");
    empty_spec(tmp.path());
    fs::write(
        tmp.path().join("spec/features/user-authentication.feature"),
        "KEEP ME\n",
    )
    .expect("write existing feature");

    // @step When I dispatch create-feature with name='User Authentication'
    let result = dispatch_command(req(tmp.path(), json!({"name": "User Authentication"})));

    // @step Then the dispatcher fails with an error containing 'File already exists: spec/features/user-authentication.feature'
    assert!(!result.success, "expected failure, got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("File already exists: spec/features/user-authentication.feature"),
        "unexpected error: {err}"
    );

    // @step And the file spec/features/user-authentication.feature still contains 'KEEP ME'
    let after = read_file(tmp.path(), "spec/features/user-authentication.feature");
    assert!(after.contains("KEEP ME"), "existing file must not be overwritten");
}

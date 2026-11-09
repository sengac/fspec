#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/create-feature-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `create-feature` (RPC-212).
// Each scenario maps to one #[test] fn with @step comments mirroring the
// Gherkin steps verbatim. The scenarios from the TypeScript-era create-feature
// feature that these tests did not prove were merged into
// create-feature-rust-port (COV-071).

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

    // @step Then the written file begins with the lines '@critical', '@component' and '@feature-group', one tag per line
    assert!(
        content.starts_with("@critical\n@component\n@feature-group\n"),
        "unexpected start:\n{content}"
    );

    // @step And the written file contains the line 'Feature: User Authentication'
    assert!(
        content.contains("Feature: User Authentication"),
        "missing Feature line:\n{content}"
    );

    // @step And the written file contains the placeholder steps '[precondition]', '[action]', and '[expected outcome]'
    assert!(
        content.contains("[precondition]"),
        "missing [precondition]:\n{content}"
    );
    assert!(content.contains("[action]"), "missing [action]:\n{content}");
    assert!(
        content.contains("[expected outcome]"),
        "missing [expected outcome]:\n{content}"
    );

    // @step And the written file ends with a trailing newline
    assert!(content.ends_with('\n'), "must end with trailing newline");

    // @step And the written file contains the line '  Architecture notes:' and the Background lines 'As a [role]', 'I want to [action]' and 'So that [benefit]'
    assert!(
        content.contains("\n  Architecture notes:\n"),
        "missing architecture notes:\n{content}"
    );
    assert!(
        content.contains(
            "  Background: User Story\n    As a [role]\n    I want to [action]\n    So that [benefit]\n"
        ),
        "missing user story Background:\n{content}"
    );
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

    let cov_raw = read_file(
        tmp.path(),
        "spec/features/user-authentication.feature.coverage",
    );
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
        data["prefillDetection"]["systemReminder"]
            .as_str()
            .is_some(),
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
        data.get("fileNamingReminder")
            .map(serde_json::Value::is_null)
            .unwrap_or(true),
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
    assert!(
        after.contains("KEEP ME"),
        "existing file must not be overwritten"
    );

    // @step And the dispatcher error contains 'Suggestion: Use a different name or delete the existing file'
    assert!(
        err.contains("Suggestion: Use a different name or delete the existing file"),
        "missing suggestion: {err}"
    );
}

fn create(project_root: &Path, name: &str) {
    let result = dispatch_command(req(project_root, json!({"name": name})));
    assert!(
        result.success,
        "create-feature {name}: expected success, got {result:?}"
    );
}

fn has_line(content: &str, line: &str) -> bool {
    content.lines().any(|l| l == line)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Feature names become kebab-case file names
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_feature_names_become_kebab_case_file_names() {
    // @step Given a project root tempdir with an empty spec directory
    let tmp = TempDir::new().expect("tempdir");
    empty_spec(tmp.path());

    // @step When I dispatch create-feature with name='Real Time Event Monitoring', then name='API/REST Endpoints', then name='Login'
    create(tmp.path(), "Real Time Event Monitoring");
    create(tmp.path(), "API/REST Endpoints");
    create(tmp.path(), "Login");

    // @step Then spec/features/real-time-event-monitoring.feature contains the line 'Feature: Real Time Event Monitoring'
    let content = read_file(
        tmp.path(),
        "spec/features/real-time-event-monitoring.feature",
    );
    assert!(
        has_line(&content, "Feature: Real Time Event Monitoring"),
        "{content}"
    );

    // @step And spec/features/api-rest-endpoints.feature contains the line 'Feature: API/REST Endpoints'
    let content = read_file(tmp.path(), "spec/features/api-rest-endpoints.feature");
    assert!(
        has_line(&content, "Feature: API/REST Endpoints"),
        "{content}"
    );

    // @step And spec/features/login.feature contains the line 'Feature: Login'
    let content = read_file(tmp.path(), "spec/features/login.feature");
    assert!(has_line(&content, "Feature: Login"), "{content}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Creates the spec/features directory when it is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_creates_the_spec_features_directory_when_it_is_missing() {
    // @step Given a project root tempdir with no spec directory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch create-feature with name='New Feature'
    let result = dispatch_command(req(tmp.path(), json!({"name": "New Feature"})));

    // @step Then the dispatcher returns a filePath ending with 'spec/features/new-feature.feature'
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    let file_path = data["filePath"].as_str().unwrap_or("");
    assert!(
        file_path.ends_with("spec/features/new-feature.feature"),
        "unexpected filePath: {file_path}"
    );

    // @step And the directory spec/features exists on disk
    assert!(tmp.path().join("spec/features").is_dir());

    // @step And the file spec/features/new-feature.feature exists on disk
    assert!(tmp
        .path()
        .join("spec/features/new-feature.feature")
        .is_file());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A created feature file passes validate
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_a_created_feature_file_passes_validate() {
    // @step Given a project root tempdir with an empty spec directory
    let tmp = TempDir::new().expect("tempdir");
    empty_spec(tmp.path());

    // @step When I dispatch create-feature with name='Shopping Cart'
    create(tmp.path(), "Shopping Cart");

    // @step And I dispatch validate with file='spec/features/shopping-cart.feature'
    let result = dispatch_command(DispatchRequest {
        command: "validate".to_string(),
        args_json: json!({"file": "spec/features/shopping-cart.feature"}).to_string(),
        project_root: tmp.path().to_path_buf(),
    });

    // @step Then validate returns exitCode=0 and invalidCount=0
    assert!(result.success, "expected validate success, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["exitCode"].as_i64(), Some(0), "got {data}");
    assert_eq!(data["invalidCount"].as_i64(), Some(0), "got {data}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: A coverage sidecar failure does not fail feature creation
// (moved from auto-create-coverage-files, COV-072)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_a_coverage_sidecar_failure_does_not_fail_feature_creation() {
    // @step Given a project root tempdir whose spec/features directory holds a directory named 'test.feature.coverage'
    let tmp = TempDir::new().expect("tempdir");
    empty_spec(tmp.path());
    fs::create_dir_all(tmp.path().join("spec/features/test.feature.coverage"))
        .expect("mkdir blocking sidecar directory");

    // @step When I dispatch create-feature with name='Test'
    let result = dispatch_command(req(tmp.path(), json!({"name": "Test"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");

    // @step And the file spec/features/test.feature exists on disk
    assert!(tmp.path().join("spec/features/test.feature").is_file());

    // @step And the result's coverageFile reports created=false and status 'error'
    assert_eq!(data["coverageFile"]["created"], json!(false), "got {data}");
    assert_eq!(data["coverageFile"]["status"], json!("error"), "got {data}");

    // @step And the result's coverageFile message starts with 'Warning: Failed to create coverage file'
    let message = data["coverageFile"]["message"].as_str().unwrap_or("");
    assert!(
        message.starts_with("Warning: Failed to create coverage file"),
        "unexpected coverage message: {message}"
    );
}

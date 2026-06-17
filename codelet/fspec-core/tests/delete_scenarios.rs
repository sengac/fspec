#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/delete-scenarios-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `delete-scenarios`
// (RPC-220). Each scenario maps to one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.
//
// PHASE B (TESTING): the core impl is still a stub, so every dispatch
// returns FspecCoreError::NotYetPorted. The behavioural tests are RED
// until PHASE C.

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
        command: "delete-scenarios".to_string(),
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
    fs::read_to_string(project_root.join(rel)).expect("read feature file")
}

/// Feature with two @spike scenarios and one untagged scenario.
fn two_spike_one_plain() -> String {
    "Feature: Demo\n\n  @spike\n  Scenario: First spike\n    Given a precondition\n\n  @spike\n  Scenario: Second spike\n    Given another precondition\n\n  Scenario: Plain keeper\n    Given an untagged precondition\n".to_string()
}

/// Feature with a scenario tagged @deprecated @critical and one tagged
/// only @deprecated.
fn and_logic_feature() -> String {
    "Feature: Demo\n\n  @deprecated @critical\n  Scenario: Both tags\n    Given a precondition\n\n  @deprecated\n  Scenario: Only deprecated\n    Given another precondition\n".to_string()
}

fn data_of(result: &codelet_fspec_core::DispatchResult) -> Value {
    serde_json::from_str(&result.data).expect("parse data json")
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

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher dry-run reports matches without modifying files
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_dry_run_reports_matches_without_modifying_files() {
    // @step Given a project root tempdir with one feature containing two @spike scenarios and one untagged scenario
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/demo.feature", &two_spike_one_plain());
    let before = read_feature(tmp.path(), "spec/features/demo.feature");

    // @step When I dispatch delete-scenarios with tags=['@spike'] and dryRun=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tags": ["@spike"], "dryRun": true}),
    ));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = data_of(&result);

    // @step Then the dispatcher returns success=true with deletedCount=2 and fileCount=1
    assert_eq!(data["success"].as_bool(), Some(true), "got data: {data}");
    assert_eq!(data["deletedCount"].as_u64(), Some(2), "got data: {data}");
    assert_eq!(data["fileCount"].as_u64(), Some(1), "got data: {data}");

    // @step And the dispatcher scenarios array lists the two matching scenarios
    assert_eq!(
        data["scenarios"].as_array().map(Vec::len),
        Some(2),
        "got data: {data}"
    );

    // @step And the feature file is unchanged on disk
    let after = read_feature(tmp.path(), "spec/features/demo.feature");
    assert_eq!(before, after, "dry-run must not modify the file");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher real delete removes matching scenarios and keeps the rest
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_real_delete_removes_matching_scenarios_and_keeps_the_rest() {
    // @step Given a project root tempdir with one feature containing two @spike scenarios and one untagged scenario
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/demo.feature", &two_spike_one_plain());

    // @step When I dispatch delete-scenarios with tags=['@spike']
    let result = dispatch_command(req(tmp.path(), json!({"tags": ["@spike"]})));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = data_of(&result);

    // @step Then the dispatcher returns success=true with deletedCount=2
    assert_eq!(data["success"].as_bool(), Some(true), "got data: {data}");
    assert_eq!(data["deletedCount"].as_u64(), Some(2), "got data: {data}");

    let after = read_feature(tmp.path(), "spec/features/demo.feature");

    // @step And the feature file no longer contains the @spike scenarios
    assert!(!after.contains("First spike"), "got:\n{after}");
    assert!(!after.contains("Second spike"), "got:\n{after}");

    // @step And the feature file still contains the untagged scenario
    assert!(after.contains("Plain keeper"), "got:\n{after}");

    // @step And the feature file re-parses as valid Gherkin
    assert!(
        after.contains("Feature: Demo") && after.contains("Scenario: Plain keeper"),
        "result must remain structurally valid Gherkin; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher applies AND logic across multiple tags
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_applies_and_logic_across_multiple_tags() {
    // @step Given a project root tempdir with one feature containing a scenario tagged @deprecated and @critical and a scenario tagged only @deprecated
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/demo.feature", &and_logic_feature());

    // @step When I dispatch delete-scenarios with tags=['@deprecated','@critical']
    let result = dispatch_command(req(
        tmp.path(),
        json!({"tags": ["@deprecated", "@critical"]}),
    ));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = data_of(&result);

    // @step Then the dispatcher returns success=true with deletedCount=1
    assert_eq!(data["success"].as_bool(), Some(true), "got data: {data}");
    assert_eq!(data["deletedCount"].as_u64(), Some(1), "got data: {data}");

    // @step And only the scenario carrying both tags is removed
    let after = read_feature(tmp.path(), "spec/features/demo.feature");
    assert!(!after.contains("Both tags"), "got:\n{after}");
    assert!(after.contains("Only deprecated"), "got:\n{after}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher reports no feature files
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_reports_no_feature_files() {
    // @step Given a project root tempdir with no spec/features directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch delete-scenarios with tags=['@spike']
    let result = dispatch_command(req(tmp.path(), json!({"tags": ["@spike"]})));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = data_of(&result);

    // @step Then the dispatcher returns success=true with deletedCount=0
    assert_eq!(data["success"].as_bool(), Some(true), "got data: {data}");
    assert_eq!(data["deletedCount"].as_u64(), Some(0), "got data: {data}");

    // @step And the dispatcher message equals 'No feature files found'
    assert_eq!(data["message"].as_str(), Some("No feature files found"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher reports no matching scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_reports_no_matching_scenarios() {
    // @step Given a project root tempdir with one feature containing only untagged scenarios
    let tmp = TempDir::new().expect("tempdir");
    write_feature(
        tmp.path(),
        "spec/features/demo.feature",
        "Feature: Demo\n\n  Scenario: Plain one\n    Given a precondition\n\n  Scenario: Plain two\n    Given another precondition\n",
    );

    // @step When I dispatch delete-scenarios with tags=['@spike']
    let result = dispatch_command(req(tmp.path(), json!({"tags": ["@spike"]})));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = data_of(&result);

    // @step Then the dispatcher returns success=true with deletedCount=0
    assert_eq!(data["success"].as_bool(), Some(true), "got data: {data}");
    assert_eq!(data["deletedCount"].as_u64(), Some(0), "got data: {data}");

    // @step And the dispatcher message equals 'No scenarios found matching tags'
    assert_eq!(
        data["message"].as_str(),
        Some("No scenarios found matching tags")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects an empty tag list
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_rejects_an_empty_tag_list() {
    // @step Given a project root tempdir with one feature tagged @spike
    let tmp = TempDir::new().expect("tempdir");
    write_feature(tmp.path(), "spec/features/demo.feature", &two_spike_one_plain());

    // @step When I dispatch delete-scenarios with tags=[]
    let result = dispatch_command(req(tmp.path(), json!({"tags": []})));

    // @step Then the dispatcher returns success=false
    // Per delete-features parity: the core returns a result object (it does not
    // throw), so the dispatch envelope succeeds and the failure is carried by
    // the inner `success:false` field of the JSON payload.
    assert!(result.success, "expected dispatch envelope success; got {result:?}");
    let data = data_of(&result);
    assert_eq!(
        data["success"].as_bool(),
        Some(false),
        "expected inner success=false; got {data}"
    );

    // @step And the dispatcher error equals 'At least one --tag is required'
    let err = dispatcher_error(&result);
    assert!(
        err.contains("At least one --tag is required"),
        "expected required-tag message; got: {err}"
    );
}

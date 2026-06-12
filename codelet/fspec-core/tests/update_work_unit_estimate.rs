#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/update-work-unit-estimate-rust-port.feature
//
// Dispatcher-level acceptance tests for the Rust port of
// `update-work-unit-estimate` (RPC-318). Each scenario maps to exactly one
// #[test] function with `@step` comments mirroring the Gherkin steps verbatim.
//
// At the end of Phase B these tests MUST fail with `NotYetPorted` because the
// supervisor has not yet wired the dispatcher to call
// `commands::update_work_unit_estimate::run`. After Phase C + supervisor wiring
// they turn green.

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
        command: "update-work-unit-estimate".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_file(path: &Path, raw: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(path, raw).expect("write file");
}

fn write_work_units(project_root: &Path, raw: &str) {
    write_file(&project_root.join("spec/work-units.json"), raw);
}

fn write_feature(project_root: &Path, file_name: &str, raw: &str) {
    write_file(&project_root.join("spec/features").join(file_name), raw);
}

fn read_work_units(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/work-units.json"))
        .expect("read spec/work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

/// A single-work-unit store keyed by id with the given type.
fn one_unit_typed(id: &str, ty: &str) -> String {
    format!(
        r#"{{
  "version": "0.7.1",
  "workUnits": {{
    "{id}": {{
      "id": "{id}", "title": "Some unit", "type": "{ty}", "status": "specifying",
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
    }}
  }},
  "states": {{
    "backlog": [], "specifying": ["{id}"], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }}
}}"#
    )
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher estimates a task without any feature file
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_estimates_task_without_feature_file() {
    // @step Given spec/work-units.json contains work unit 'TASK-001' of type 'task'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &one_unit_typed("TASK-001", "task"));

    // @step When I dispatch update-work-unit-estimate with workUnitId='TASK-001' and estimate=3
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "TASK-001", "estimate": 3 }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And spec/work-units.json work unit 'TASK-001' has estimate 3
    let on_disk = read_work_units(tmp.path());
    assert_eq!(on_disk["workUnits"]["TASK-001"]["estimate"].as_i64(), Some(3));

    // @step And the updatedAt of 'TASK-001' is set to a non-empty ISO-8601 string
    let updated_at = on_disk["workUnits"]["TASK-001"]["updatedAt"]
        .as_str()
        .expect("updatedAt present");
    assert!(!updated_at.is_empty(), "updatedAt must be non-empty");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects a non-Fibonacci estimate
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_rejects_non_fibonacci_estimate() {
    // @step Given spec/work-units.json contains work unit 'TASK-001' of type 'task'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &one_unit_typed("TASK-001", "task"));

    // @step When I dispatch update-work-unit-estimate with workUnitId='TASK-001' and estimate=7
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "TASK-001", "estimate": 7 }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error message contains the substring 'Failed to update work unit estimate'
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Failed to update work unit estimate"),
        "error must contain wrap prefix; got: {msg}"
    );

    // @step And the error message contains the substring 'Invalid estimate: 7. Must be one of: 1,2,3,5,8,13,21'
    assert!(
        msg.contains("Invalid estimate: 7. Must be one of: 1,2,3,5,8,13,21"),
        "error must mention invalid estimate; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects a missing work unit
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_rejects_missing_work_unit() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch update-work-unit-estimate with workUnitId='MISSING-999' and estimate=5
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "MISSING-999", "estimate": 5 }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error message contains the substring 'Work unit MISSING-999 not found'
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Work unit MISSING-999 not found"),
        "error must mention missing work unit; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher blocks estimating a story without a feature file
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_blocks_story_without_feature_file() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' of type 'story'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &one_unit_typed("AUTH-001", "story"));

    // @step And spec/features contains no feature file tagged @AUTH-001
    // (no spec/features directory created at all)

    // @step When I dispatch update-work-unit-estimate with workUnitId='AUTH-001' and estimate=5
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "estimate": 5 }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error message contains the substring 'ACDD VIOLATION'
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("ACDD VIOLATION"),
        "error must mention ACDD VIOLATION; got: {msg}"
    );

    // @step And the error message contains the substring 'without completed feature file'
    assert!(
        msg.contains("without completed feature file"),
        "error must mention missing feature file; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher blocks estimating a story whose feature file has prefill placeholders
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_blocks_story_with_prefill_placeholders() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' of type 'story'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &one_unit_typed("AUTH-001", "story"));

    // @step And spec/features contains a feature file tagged @AUTH-001 that contains a role-placeholder token
    write_feature(
        tmp.path(),
        "auth.feature",
        "@AUTH-001\nFeature: Auth\n\n  Background:\n    As a [role]\n    I want to log in\n",
    );

    // @step When I dispatch update-work-unit-estimate with workUnitId='AUTH-001' and estimate=5
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "estimate": 5 }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error message contains the substring 'ACDD VIOLATION'
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("ACDD VIOLATION"),
        "error must mention ACDD VIOLATION; got: {msg}"
    );

    // @step And the error message contains the substring 'prefill placeholders'
    assert!(
        msg.contains("prefill placeholders"),
        "error must mention prefill placeholders; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher estimates a story with a clean tagged feature file
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_estimates_story_with_clean_feature_file() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' of type 'story'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &one_unit_typed("AUTH-001", "story"));

    // @step And spec/features contains a clean feature file tagged @AUTH-001 with no prefill placeholders
    write_feature(
        tmp.path(),
        "auth.feature",
        "@AUTH-001\nFeature: Auth\n\n  Scenario: Login\n    Given I am on the login page\n    When I sign in\n    Then I see the dashboard\n",
    );

    // @step When I dispatch update-work-unit-estimate with workUnitId='AUTH-001' and estimate=5
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "estimate": 5 }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And spec/work-units.json work unit 'AUTH-001' has estimate 5
    let on_disk = read_work_units(tmp.path());
    assert_eq!(on_disk["workUnits"]["AUTH-001"]["estimate"].as_i64(), Some(5));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec-core function as the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_is_the_single_source_of_truth_for_estimate() {
    // @step Given spec/work-units.json contains work unit 'TASK-001' of type 'task'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &one_unit_typed("TASK-001", "task"));

    // @step When I dispatch update-work-unit-estimate via the dispatcher with workUnitId='TASK-001' and estimate=3
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "TASK-001", "estimate": 3 }),
    ));

    // @step And I run `./codelet/target/release/fspec update-work-unit-estimate TASK-001 3` in an identical workspace
    // (The CLI path is exercised by codelet/fspec/tests/cli_update_work_unit_estimate.rs; here we assert the
    //  dispatcher path — the single source of truth both front doors converge on — succeeds.)

    // @step Then both invocations produce the same success result and the same on-disk estimate 3
    assert!(result.success, "dispatcher path must succeed; got {result:?}");
    let on_disk = read_work_units(tmp.path());
    assert_eq!(on_disk["workUnits"]["TASK-001"]["estimate"].as_i64(), Some(3));
}

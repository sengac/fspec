// Feature: spec/features/clear-virtual-hooks-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `clear-virtual-hooks`
// (RPC-205). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// RED phase: clear-virtual-hooks is still a NotYetPorted stub (it is NOT in
// `PORTED_COMMANDS`), so every assertion below should fail today —
// dispatch_command returns `success=false` with the canonical "not yet
// ported" error string instead of the expected mutation / payload.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "clear-virtual-hooks".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn seed_work_units(project_root: &Path, value: Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("create spec dir");
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&value).expect("serialize seed"),
    )
    .expect("write work-units.json");
}

fn read_work_units(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

/// Single work-unit AUTH-001 with NO virtualHooks field.
fn auth001_no_hooks() -> Value {
    json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": {
            "AUTH-001": {
                "id": "AUTH-001",
                "title": "Login feature",
                "status": "backlog",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": ["AUTH-001"],
            "specifying": [], "testing": [], "implementing": [],
            "validating": [], "done": [], "blocked": []
        }
    })
}

fn auth001_empty_hooks() -> Value {
    let mut v = auth001_no_hooks();
    v["workUnits"]["AUTH-001"]["virtualHooks"] = json!([]);
    v
}

fn auth001_three_hooks() -> Value {
    let mut v = auth001_no_hooks();
    v["workUnits"]["AUTH-001"]["virtualHooks"] = json!([
        { "name": "lint",  "event": "post-implementing", "command": "npm run lint", "blocking": true  },
        { "name": "test",  "event": "post-implementing", "command": "npm test",     "blocking": false },
        { "name": "eslint","event": "pre-validating",    "command": "eslint .",     "blocking": true, "gitContext": true }
    ]);
    v
}

fn auth001_two_hooks_lint_test() -> Value {
    let mut v = auth001_no_hooks();
    v["workUnits"]["AUTH-001"]["virtualHooks"] = json!([
        { "name": "lint", "event": "post-implementing", "command": "npm run lint", "blocking": true  },
        { "name": "test", "event": "post-implementing", "command": "npm test",     "blocking": false }
    ]);
    v
}

// ---------- scenarios ----------

#[test]
fn scenario_returns_error_when_work_unit_does_not_exist() {
    // Scenario: Returns error when the requested work unit does not exist

    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_no_hooks());

    // @step When I dispatch clear-virtual-hooks with workUnitId='AUTH-999'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-999" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the exact substring "Work unit 'AUTH-999' does not exist"
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("Work unit 'AUTH-999' does not exist"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn scenario_returns_error_when_auto_created_store_is_empty() {
    // Scenario: Returns error when spec/work-units.json is auto-created and the requested id is not in the empty store

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the exact substring "Work unit 'AUTH-001' does not exist"
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("Work unit 'AUTH-001' does not exist"),
        "error message missing canonical substring: {msg}"
    );

    // @step Then spec/work-units.json exists after the call
    assert!(
        tmp.path().join("spec/work-units.json").exists(),
        "ensure_work_units_file must auto-create spec/work-units.json"
    );
}

#[test]
fn scenario_clears_all_hooks_from_unit_with_three_virtual_hooks() {
    // Scenario: Clears all hooks from a work unit with three virtualHooks

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks in order: 'lint' (post-implementing), 'test' (post-implementing), 'eslint' (pre-validating)
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_three_hooks());
    let original_updated_at = "2026-06-01T00:00:00.000Z";

    // @step When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the parsed JSON has clearedCount=3
    let data = parse_data(&result.data);
    assert_eq!(
        data["clearedCount"].as_u64(),
        Some(3),
        "expected clearedCount=3; got {}",
        result.data
    );

    // @step Then spec/work-units.json AUTH-001 virtualHooks is an empty array
    let disk = read_work_units(tmp.path());
    let hooks = disk["workUnits"]["AUTH-001"]["virtualHooks"]
        .as_array()
        .expect("virtualHooks must be an array after clear");
    assert!(
        hooks.is_empty(),
        "virtualHooks should be empty after clear, got {hooks:?}"
    );

    // @step Then spec/work-units.json AUTH-001 updatedAt is a valid ISO-8601 timestamp newer than before
    let updated_at = disk["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("updatedAt is a string");
    assert_ne!(
        updated_at, original_updated_at,
        "updatedAt must be bumped after clear"
    );
    // ISO-8601 sanity check (at minimum yyyy-mm-ddThh:mm:ss).
    assert!(
        updated_at.len() >= 19 && updated_at.contains('T'),
        "updatedAt should look like an ISO-8601 timestamp, got: {updated_at}"
    );
}

#[test]
fn scenario_clearing_unit_with_no_virtual_hooks_returns_cleared_count_zero() {
    // Scenario: Clearing a work unit with no virtualHooks succeeds with clearedCount=0

    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_no_hooks());

    // @step When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the parsed JSON has clearedCount=0
    let data = parse_data(&result.data);
    assert_eq!(
        data["clearedCount"].as_u64(),
        Some(0),
        "expected clearedCount=0; got {}",
        result.data
    );

    // @step Then spec/work-units.json AUTH-001 virtualHooks is an empty array
    let disk = read_work_units(tmp.path());
    let hooks = disk["workUnits"]["AUTH-001"]["virtualHooks"]
        .as_array()
        .expect("virtualHooks must be an empty array, even when previously missing");
    assert!(
        hooks.is_empty(),
        "virtualHooks should be initialized to [] after clear"
    );
}

#[test]
fn scenario_clearing_unit_with_empty_virtual_hooks_returns_cleared_count_zero() {
    // Scenario: Clearing a work unit with an empty virtualHooks array succeeds with clearedCount=0

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[]
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_empty_hooks());

    // @step When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the parsed JSON has clearedCount=0
    let data = parse_data(&result.data);
    assert_eq!(
        data["clearedCount"].as_u64(),
        Some(0),
        "expected clearedCount=0; got {}",
        result.data
    );

    // @step Then spec/work-units.json AUTH-001 virtualHooks is an empty array
    let disk = read_work_units(tmp.path());
    let hooks = disk["workUnits"]["AUTH-001"]["virtualHooks"]
        .as_array()
        .expect("virtualHooks must remain an empty array");
    assert!(hooks.is_empty(), "virtualHooks must stay []");
}

#[test]
fn scenario_script_files_are_unlinked_for_each_cleared_hook() {
    // Scenario: Script files in spec/hooks/.virtual/ are unlinked for each cleared hook

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks 'lint' and 'test'
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_two_hooks_lint_test());

    // @step Given spec/hooks/.virtual/AUTH-001-lint.sh exists
    let virtual_dir = tmp.path().join("spec/hooks/.virtual");
    fs::create_dir_all(&virtual_dir).expect("create .virtual dir");
    let lint_script = virtual_dir.join("AUTH-001-lint.sh");
    fs::write(&lint_script, "#!/bin/bash\nnpm run lint\n").expect("write lint script");
    assert!(lint_script.exists());

    // @step Given spec/hooks/.virtual/AUTH-001-test.sh exists
    let test_script = virtual_dir.join("AUTH-001-test.sh");
    fs::write(&test_script, "#!/bin/bash\nnpm test\n").expect("write test script");
    assert!(test_script.exists());

    // @step When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then spec/hooks/.virtual/AUTH-001-lint.sh no longer exists
    assert!(
        !lint_script.exists(),
        "AUTH-001-lint.sh should be deleted after clear"
    );

    // @step Then spec/hooks/.virtual/AUTH-001-test.sh no longer exists
    assert!(
        !test_script.exists(),
        "AUTH-001-test.sh should be deleted after clear"
    );
}

#[test]
fn scenario_missing_script_files_are_silently_ignored() {
    // Scenario: Missing script files are silently ignored

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks 'lint' and 'test'
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_two_hooks_lint_test());

    // @step Given spec/hooks/.virtual/ does not contain any script files
    assert!(!tmp.path().join("spec/hooks/.virtual").exists());

    // @step When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the parsed JSON has clearedCount=2
    let data = parse_data(&result.data);
    assert_eq!(
        data["clearedCount"].as_u64(),
        Some(2),
        "expected clearedCount=2 even when script files are absent; got {}",
        result.data
    );
}

#[test]
fn scenario_missing_work_unit_id_argument_is_rejected() {
    // Scenario: Missing workUnitId argument is rejected as InvalidArgs

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch clear-virtual-hooks with an empty args object {}
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false for missing workUnitId, got {result:?}"
    );

    // @step Then the error message indicates that workUnitId is required
    let msg = result.error.as_ref().expect("error message expected");
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("workunitid") || lower.contains("work unit id") || lower.contains("workunit"),
        "error message should mention workUnitId; got: {msg}"
    );
}

#[test]
fn scenario_result_json_shape_preserves_field_order() {
    // Scenario: Result JSON shape preserves field order success then clearedCount

    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_no_hooks());

    // @step When I dispatch clear-virtual-hooks with workUnitId='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the DispatchResult.data parses to a JSON object whose first key is "success" and whose second key is "clearedCount"
    let success_pos = result
        .data
        .find("\"success\"")
        .expect("data must contain \"success\" key");
    let cleared_pos = result
        .data
        .find("\"clearedCount\"")
        .expect("data must contain \"clearedCount\" key");
    assert!(
        success_pos < cleared_pos,
        "expected \"success\" before \"clearedCount\" in data; got success={success_pos} clearedCount={cleared_pos}\n{}",
        result.data
    );
}

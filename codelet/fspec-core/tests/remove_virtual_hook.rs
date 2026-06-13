// Feature: spec/features/remove-virtual-hook-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `remove-virtual-hook`
// (RPC-283). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// RED phase: remove-virtual-hook is still a NotYetPorted stub (it is NOT in
// `PORTED_COMMANDS`), so every assertion below should fail today —
// dispatch_command returns `success=false` with the canonical "not yet
// ported" error string instead of the expected payload.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-virtual-hook".to_string(),
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

fn auth001_base() -> Value {
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

fn auth001_with_hooks(hooks: Value) -> Value {
    let mut v = auth001_base();
    v["workUnits"]["AUTH-001"]["virtualHooks"] = hooks;
    v
}

// ---------- scenarios ----------

#[test]
fn scenario_removes_the_only_hook_and_returns_remaining_count_zero() {
    // Scenario: Removes the only hook and returns remainingCount=0

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',event:'post-implementing',command:'eslint .',blocking:true}]
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        auth001_with_hooks(json!([
            {
                "name": "eslint",
                "event": "post-implementing",
                "command": "eslint .",
                "blocking": true
            }
        ])),
    );

    // @step When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='eslint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "hookName": "eslint" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the parsed JSON has remainingCount=0
    let data = parse_data(&result.data);
    assert_eq!(data["remainingCount"].as_u64(), Some(0));

    // @step And the on-disk virtualHooks array has length 0
    let v = read_work_units(tmp.path());
    let hooks = v["workUnits"]["AUTH-001"]["virtualHooks"]
        .as_array()
        .expect("virtualHooks array");
    assert!(
        hooks.is_empty(),
        "expected empty virtualHooks; got {hooks:?}"
    );
}

#[test]
fn scenario_removing_middle_entry_preserves_remaining_order() {
    // Scenario: Removing a middle entry preserves the order of remaining hooks

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[lint,test,eslint] in that order
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        auth001_with_hooks(json!([
            { "name": "lint",   "event": "post-implementing", "command": "npm run lint", "blocking": true },
            { "name": "test",   "event": "post-implementing", "command": "npm test",     "blocking": false },
            { "name": "eslint", "event": "pre-validating",    "command": "eslint .",     "blocking": true }
        ])),
    );

    // @step When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='lint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "hookName": "lint" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the parsed JSON has remainingCount=2
    let data = parse_data(&result.data);
    assert_eq!(data["remainingCount"].as_u64(), Some(2));

    // @step And the on-disk virtualHooks names in order are ['test','eslint']
    let v = read_work_units(tmp.path());
    let names: Vec<&str> = v["workUnits"]["AUTH-001"]["virtualHooks"]
        .as_array()
        .expect("virtualHooks array")
        .iter()
        .map(|h| h["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, vec!["test", "eslint"]);
}

#[test]
fn scenario_removing_hook_with_duplicate_names_removes_all_matches() {
    // Scenario: Removing a hook with duplicate names removes ALL matches (filter semantics)

    // @step Given spec/work-units.json contains AUTH-001 with two virtualHooks both named 'lint' plus one named 'test'
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        auth001_with_hooks(json!([
            { "name": "lint", "event": "post-implementing", "command": "npm run lint", "blocking": true },
            { "name": "lint", "event": "pre-validating",    "command": "eslint .",     "blocking": false },
            { "name": "test", "event": "post-implementing", "command": "npm test",     "blocking": false }
        ])),
    );

    // @step When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='lint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "hookName": "lint" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the parsed JSON has remainingCount=1
    let data = parse_data(&result.data);
    assert_eq!(data["remainingCount"].as_u64(), Some(1));

    // @step And the on-disk virtualHooks names in order are ['test']
    let v = read_work_units(tmp.path());
    let names: Vec<&str> = v["workUnits"]["AUTH-001"]["virtualHooks"]
        .as_array()
        .expect("virtualHooks array")
        .iter()
        .map(|h| h["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, vec!["test"]);
}

#[test]
fn scenario_removing_hook_deletes_associated_script_file() {
    // Scenario: Removing a hook with an associated script deletes the script file

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',...,command:'spec/hooks/.virtual/AUTH-001-eslint.sh',blocking:true,gitContext:true}]
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        auth001_with_hooks(json!([
            {
                "name": "eslint",
                "event": "post-implementing",
                "command": "spec/hooks/.virtual/AUTH-001-eslint.sh",
                "blocking": true,
                "gitContext": true
            }
        ])),
    );

    // @step And the file spec/hooks/.virtual/AUTH-001-eslint.sh exists on disk
    let script_dir = tmp.path().join("spec/hooks/.virtual");
    fs::create_dir_all(&script_dir).expect("mkdir .virtual");
    let script_path = script_dir.join("AUTH-001-eslint.sh");
    fs::write(&script_path, "#!/bin/bash\necho hi\n").expect("write script");
    assert!(script_path.exists());

    // @step When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='eslint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "hookName": "eslint" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the file spec/hooks/.virtual/AUTH-001-eslint.sh no longer exists
    assert!(
        !script_path.exists(),
        "script must be cleaned up; still exists: {}",
        script_path.display()
    );
}

#[test]
fn scenario_removing_hook_without_script_succeeds_silently() {
    // Scenario: Removing a hook whose script file does not exist succeeds silently

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',...,blocking:true}]
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        auth001_with_hooks(json!([
            {
                "name": "eslint",
                "event": "post-implementing",
                "command": "eslint .",
                "blocking": true
            }
        ])),
    );

    // @step And spec/hooks/.virtual/AUTH-001-eslint.sh does NOT exist on disk
    assert!(!tmp
        .path()
        .join("spec/hooks/.virtual/AUTH-001-eslint.sh")
        .exists());

    // @step When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='eslint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "hookName": "eslint" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the parsed JSON has remainingCount=0
    let data = parse_data(&result.data);
    assert_eq!(data["remainingCount"].as_u64(), Some(0));
}

#[test]
fn scenario_unknown_work_unit_returns_invalid_args_with_canonical_message() {
    // Scenario: Unknown work unit returns InvalidArgs with the canonical message

    // @step Given spec/work-units.json contains AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_base());

    // @step When I dispatch remove-virtual-hook with workUnitId='AUTH-999' hookName='eslint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-999", "hookName": "eslint" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step And the error message contains the exact substring "Work unit 'AUTH-999' does not exist"
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("Work unit 'AUTH-999' does not exist"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn scenario_work_unit_without_virtual_hooks_field_returns_invalid_args() {
    // Scenario: Work unit without virtualHooks field returns InvalidArgs

    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_base());

    // @step When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='eslint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "hookName": "eslint" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step And the error message contains the exact substring "No virtual hooks configured for AUTH-001"
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("No virtual hooks configured for AUTH-001"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn scenario_empty_virtual_hooks_array_also_returns_invalid_args() {
    // Scenario: Work unit with empty virtualHooks array also returns InvalidArgs

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[]
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_with_hooks(json!([])));

    // @step When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='eslint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "hookName": "eslint" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step And the error message contains the exact substring "No virtual hooks configured for AUTH-001"
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("No virtual hooks configured for AUTH-001"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn scenario_non_matching_hook_name_returns_invalid_args() {
    // Scenario: Non-matching hookName returns InvalidArgs naming both the hook and the work unit

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',...,blocking:true}]
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        auth001_with_hooks(json!([
            {
                "name": "eslint",
                "event": "post-implementing",
                "command": "eslint .",
                "blocking": true
            }
        ])),
    );

    // @step When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='missing'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "hookName": "missing" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step And the error message contains the exact substring "Virtual hook 'missing' not found in AUTH-001"
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("Virtual hook 'missing' not found in AUTH-001"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn scenario_empty_args_object_rejected_mentioning_work_unit_id() {
    // Scenario: Empty args object is rejected as InvalidArgs mentioning missing workUnitId

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch remove-virtual-hook with an empty args object {}
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step And the error message indicates that workUnitId is required
    let msg = result.error.as_ref().expect("error message expected");
    let lower = msg.to_lowercase();
    assert!(
        lower.contains("workunitid")
            || lower.contains("work unit id")
            || lower.contains("workunit"),
        "error message should mention workUnitId; got: {msg}"
    );
}

#[test]
fn scenario_result_json_uses_camel_case_remaining_count_key() {
    // Scenario: Result JSON uses camelCase remainingCount key

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',...,blocking:true}]
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        auth001_with_hooks(json!([
            {
                "name": "eslint",
                "event": "post-implementing",
                "command": "eslint .",
                "blocking": true
            }
        ])),
    );

    // @step When I dispatch remove-virtual-hook with workUnitId='AUTH-001' hookName='eslint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "hookName": "eslint" }),
    ));

    // @step Then the DispatchResult.data parses to a JSON object containing the key 'remainingCount'
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    let obj = data.as_object().expect("data should be object");
    assert!(
        obj.contains_key("remainingCount"),
        "data must contain key 'remainingCount'; got: {result:?}"
    );

    // @step And the DispatchResult.data does NOT contain the key 'remaining_count'
    assert!(
        !obj.contains_key("remaining_count"),
        "data must NOT contain snake_case 'remaining_count'; got: {result:?}"
    );

    // @step And the DispatchResult.data contains 'success' equal to true
    assert_eq!(obj.get("success"), Some(&Value::Bool(true)));
}

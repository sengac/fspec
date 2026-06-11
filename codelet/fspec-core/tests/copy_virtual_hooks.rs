// Feature: spec/features/copy-virtual-hooks-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `copy-virtual-hooks`
// (RPC-209). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// RED phase: copy-virtual-hooks is still a NotYetPorted stub (it is NOT in
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
        command: "copy-virtual-hooks".to_string(),
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

fn lint_hook() -> Value {
    json!({ "name": "lint", "event": "post-implementing", "command": "npm run lint", "blocking": true })
}
fn test_hook() -> Value {
    json!({ "name": "test", "event": "post-implementing", "command": "npm test", "blocking": false })
}
fn eslint_hook() -> Value {
    json!({ "name": "eslint", "event": "pre-validating", "command": "eslint .", "blocking": true, "gitContext": true })
}
fn old_hook() -> Value {
    json!({ "name": "old-hook", "event": "post-implementing", "command": "echo legacy", "blocking": false })
}

/// Build a work-units.json with the given list of (id, virtualHooks?) pairs.
/// A None entry means the unit has no virtualHooks field at all.
fn build_store(units: &[(&str, Option<Vec<Value>>)]) -> Value {
    let mut wus = serde_json::Map::new();
    let mut backlog = Vec::new();
    for (id, hooks) in units {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), json!(id));
        obj.insert("title".into(), json!(format!("title for {id}")));
        obj.insert("status".into(), json!("backlog"));
        obj.insert("createdAt".into(), json!("2026-06-01T00:00:00.000Z"));
        obj.insert("updatedAt".into(), json!("2026-06-01T00:00:00.000Z"));
        if let Some(h) = hooks {
            obj.insert("virtualHooks".into(), Value::Array(h.clone()));
        }
        wus.insert((*id).to_string(), Value::Object(obj));
        backlog.push(Value::String((*id).to_string()));
    }
    json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": Value::Object(wus),
        "states": {
            "backlog": Value::Array(backlog),
            "specifying": [], "testing": [], "implementing": [],
            "validating": [], "done": [], "blocked": []
        }
    })
}

// ---------- scenarios ----------

#[test]
fn scenario_copies_all_source_hooks_into_empty_target() {
    // Scenario: Copies all source hooks into an empty target

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks in order: 'lint' (post-implementing), 'test' (post-implementing), 'eslint' (pre-validating)
    let tmp = TempDir::new().expect("tempdir");
    let store = build_store(&[
        ("AUTH-001", Some(vec![lint_hook(), test_hook(), eslint_hook()])),
        // @step Given spec/work-units.json contains AUTH-002 with no virtualHooks field
        ("AUTH-002", None),
    ]);
    seed_work_units(tmp.path(), store);
    let original_source_updated_at = "2026-06-01T00:00:00.000Z";
    let original_target_updated_at = "2026-06-01T00:00:00.000Z";

    // @step When I dispatch copy-virtual-hooks with from='AUTH-001' and to='AUTH-002'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "from": "AUTH-001", "to": "AUTH-002" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the parsed JSON has copiedCount=3
    let data = parse_data(&result.data);
    assert_eq!(
        data["copiedCount"].as_u64(),
        Some(3),
        "expected copiedCount=3; got {}",
        result.data
    );

    // @step Then spec/work-units.json AUTH-002 virtualHooks contains the names ['lint','test','eslint'] in that order
    let disk = read_work_units(tmp.path());
    let target_hooks = disk["workUnits"]["AUTH-002"]["virtualHooks"]
        .as_array()
        .expect("target virtualHooks must be an array after copy");
    assert_eq!(target_hooks.len(), 3);
    assert_eq!(target_hooks[0]["name"].as_str(), Some("lint"));
    assert_eq!(target_hooks[1]["name"].as_str(), Some("test"));
    assert_eq!(target_hooks[2]["name"].as_str(), Some("eslint"));

    // @step Then spec/work-units.json AUTH-001 virtualHooks is unchanged (same length and names)
    let source_hooks = disk["workUnits"]["AUTH-001"]["virtualHooks"]
        .as_array()
        .expect("source virtualHooks must remain an array");
    assert_eq!(source_hooks.len(), 3);
    assert_eq!(source_hooks[0]["name"].as_str(), Some("lint"));
    assert_eq!(source_hooks[1]["name"].as_str(), Some("test"));
    assert_eq!(source_hooks[2]["name"].as_str(), Some("eslint"));

    // @step Then spec/work-units.json AUTH-002 updatedAt is newer than its prior value
    let target_updated = disk["workUnits"]["AUTH-002"]["updatedAt"]
        .as_str()
        .expect("target updatedAt is a string");
    assert_ne!(
        target_updated, original_target_updated_at,
        "target updatedAt must be bumped after copy"
    );

    // @step Then spec/work-units.json AUTH-001 updatedAt is NOT bumped
    let source_updated = disk["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("source updatedAt is a string");
    assert_eq!(
        source_updated, original_source_updated_at,
        "source updatedAt must NOT change during copy"
    );
}

#[test]
fn scenario_copies_only_named_hook_when_hook_name_supplied() {
    // Scenario: Copies only the named hook when hookName is supplied

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks 'lint', 'test', 'eslint'
    let tmp = TempDir::new().expect("tempdir");
    let store = build_store(&[
        ("AUTH-001", Some(vec![lint_hook(), test_hook(), eslint_hook()])),
        // @step Given spec/work-units.json contains AUTH-002 with no virtualHooks
        ("AUTH-002", None),
    ]);
    seed_work_units(tmp.path(), store);

    // @step When I dispatch copy-virtual-hooks with from='AUTH-001' and to='AUTH-002' and hookName='eslint'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "from": "AUTH-001", "to": "AUTH-002", "hookName": "eslint" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the parsed JSON has copiedCount=1
    let data = parse_data(&result.data);
    assert_eq!(
        data["copiedCount"].as_u64(),
        Some(1),
        "expected copiedCount=1; got {}",
        result.data
    );

    // @step Then spec/work-units.json AUTH-002 virtualHooks contains a single entry with name='eslint'
    let disk = read_work_units(tmp.path());
    let target_hooks = disk["workUnits"]["AUTH-002"]["virtualHooks"]
        .as_array()
        .expect("target virtualHooks must be an array");
    assert_eq!(target_hooks.len(), 1, "expected exactly one copied hook");
    assert_eq!(target_hooks[0]["name"].as_str(), Some("eslint"));
}

#[test]
fn scenario_copied_hooks_are_appended_after_existing_target_hooks() {
    // Scenario: Copied hooks are APPENDED after existing target hooks (existing entries preserved)

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks 'lint' and 'test'
    let tmp = TempDir::new().expect("tempdir");
    let store = build_store(&[
        ("AUTH-001", Some(vec![lint_hook(), test_hook()])),
        // @step Given spec/work-units.json contains AUTH-002 with virtualHook 'old-hook' already configured
        ("AUTH-002", Some(vec![old_hook()])),
    ]);
    seed_work_units(tmp.path(), store);

    // @step When I dispatch copy-virtual-hooks with from='AUTH-001' and to='AUTH-002'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "from": "AUTH-001", "to": "AUTH-002" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then spec/work-units.json AUTH-002 virtualHooks contains names ['old-hook','lint','test'] in that order
    let disk = read_work_units(tmp.path());
    let target_hooks = disk["workUnits"]["AUTH-002"]["virtualHooks"]
        .as_array()
        .expect("target virtualHooks must be an array");
    assert_eq!(target_hooks.len(), 3, "expected existing + 2 copied");
    assert_eq!(target_hooks[0]["name"].as_str(), Some("old-hook"));
    assert_eq!(target_hooks[1]["name"].as_str(), Some("lint"));
    assert_eq!(target_hooks[2]["name"].as_str(), Some("test"));
}

#[test]
fn scenario_source_work_unit_missing_returns_source_error() {
    // Scenario: Source work unit missing returns the canonical source error

    // @step Given spec/work-units.json contains AUTH-002 only
    let tmp = TempDir::new().expect("tempdir");
    let store = build_store(&[("AUTH-002", None)]);
    seed_work_units(tmp.path(), store);

    // @step When I dispatch copy-virtual-hooks with from='MISSING-001' and to='AUTH-002'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "from": "MISSING-001", "to": "AUTH-002" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the exact substring "Source work unit 'MISSING-001' does not exist"
    let msg = result.error.as_ref().expect("error expected");
    assert!(
        msg.contains("Source work unit 'MISSING-001' does not exist"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn scenario_target_work_unit_missing_returns_target_error() {
    // Scenario: Target work unit missing returns the canonical target error

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks 'lint'
    let tmp = TempDir::new().expect("tempdir");
    let store = build_store(&[("AUTH-001", Some(vec![lint_hook()]))]);
    seed_work_units(tmp.path(), store);

    // @step When I dispatch copy-virtual-hooks with from='AUTH-001' and to='MISSING-002'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "from": "AUTH-001", "to": "MISSING-002" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the exact substring "Target work unit 'MISSING-002' does not exist"
    let msg = result.error.as_ref().expect("error expected");
    assert!(
        msg.contains("Target work unit 'MISSING-002' does not exist"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn scenario_source_with_no_virtual_hooks_returns_no_hooks_configured() {
    // Scenario: Source with no virtualHooks returns the no-hooks-configured error

    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks
    // @step Given spec/work-units.json contains AUTH-002 with no virtualHooks
    let tmp = TempDir::new().expect("tempdir");
    let store = build_store(&[("AUTH-001", None), ("AUTH-002", None)]);
    seed_work_units(tmp.path(), store);

    // @step When I dispatch copy-virtual-hooks with from='AUTH-001' and to='AUTH-002'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "from": "AUTH-001", "to": "AUTH-002" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the exact substring "No virtual hooks configured for source work unit AUTH-001"
    let msg = result.error.as_ref().expect("error expected");
    assert!(
        msg.contains("No virtual hooks configured for source work unit AUTH-001"),
        "error message missing canonical substring (note: NO single quotes around id): {msg}"
    );
}

#[test]
fn scenario_hook_name_not_in_source_returns_hook_not_found_error() {
    // Scenario: hookName not present in source returns the hook-not-found error

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks 'lint' and 'test'
    // @step Given spec/work-units.json contains AUTH-002 with no virtualHooks
    let tmp = TempDir::new().expect("tempdir");
    let store = build_store(&[
        ("AUTH-001", Some(vec![lint_hook(), test_hook()])),
        ("AUTH-002", None),
    ]);
    seed_work_units(tmp.path(), store);

    // @step When I dispatch copy-virtual-hooks with from='AUTH-001' and to='AUTH-002' and hookName='missing'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "from": "AUTH-001", "to": "AUTH-002", "hookName": "missing" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the exact substring "Hook 'missing' not found in AUTH-001"
    let msg = result.error.as_ref().expect("error expected");
    assert!(
        msg.contains("Hook 'missing' not found in AUTH-001"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn scenario_missing_from_argument_is_rejected() {
    // Scenario: Missing from argument is rejected as InvalidArgs

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch copy-virtual-hooks with to='AUTH-002' and no from key
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "to": "AUTH-002" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false for missing from, got {result:?}"
    );

    // @step Then the error message indicates that --from option is required
    let msg = result.error.as_ref().expect("error expected");
    assert!(
        msg.contains("--from option is required"),
        "error message should mention '--from option is required'; got: {msg}"
    );
}

#[test]
fn scenario_missing_to_argument_is_rejected() {
    // Scenario: Missing to argument is rejected as InvalidArgs

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch copy-virtual-hooks with from='AUTH-001' and no to key
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "from": "AUTH-001" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false for missing to, got {result:?}"
    );

    // @step Then the error message indicates that --to option is required
    let msg = result.error.as_ref().expect("error expected");
    assert!(
        msg.contains("--to option is required"),
        "error message should mention '--to option is required'; got: {msg}"
    );
}

#[test]
fn scenario_result_json_shape_preserves_field_order() {
    // Scenario: Result JSON shape preserves field order success then copiedCount

    // @step Given spec/work-units.json contains AUTH-001 with virtualHook 'lint' and AUTH-002 with no hooks
    let tmp = TempDir::new().expect("tempdir");
    let store = build_store(&[
        ("AUTH-001", Some(vec![lint_hook()])),
        ("AUTH-002", None),
    ]);
    seed_work_units(tmp.path(), store);

    // @step When I dispatch copy-virtual-hooks with from='AUTH-001' and to='AUTH-002'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "from": "AUTH-001", "to": "AUTH-002" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the DispatchResult.data parses to a JSON object whose first key is "success" and whose second key is "copiedCount"
    let success_pos = result
        .data
        .find("\"success\"")
        .expect("data must contain \"success\" key");
    let copied_pos = result
        .data
        .find("\"copiedCount\"")
        .expect("data must contain \"copiedCount\" key");
    assert!(
        success_pos < copied_pos,
        "expected \"success\" before \"copiedCount\" in data; got success={success_pos} copiedCount={copied_pos}\n{}",
        result.data
    );
}

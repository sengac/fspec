// Feature: spec/features/list-virtual-hooks-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `list-virtual-hooks`
// (RPC-252). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// RED phase: list-virtual-hooks is still a NotYetPorted stub (it is NOT in
// `PORTED_COMMANDS`), so every assertion below should fail today —
// dispatch_command returns `success=false` with the canonical "not yet
// ported" error string instead of the expected payload / text rendering.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "list-virtual-hooks".to_string(),
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

/// Single work-unit AUTH-001 with an empty `virtualHooks: []` field.
fn auth001_empty_hooks() -> Value {
    let mut v = auth001_no_hooks();
    v["workUnits"]["AUTH-001"]["virtualHooks"] = json!([]);
    v
}

/// Single work-unit AUTH-001 with three virtualHooks across two events
/// (post-implementing first with two hooks, pre-validating second with one).
fn auth001_three_hooks() -> Value {
    let mut v = auth001_no_hooks();
    v["workUnits"]["AUTH-001"]["virtualHooks"] = json!([
        {
            "name": "lint",
            "event": "post-implementing",
            "command": "npm run lint",
            "blocking": true
        },
        {
            "name": "test",
            "event": "post-implementing",
            "command": "npm test",
            "blocking": false
        },
        {
            "name": "eslint",
            "event": "pre-validating",
            "command": "eslint .",
            "blocking": true,
            "gitContext": true
        }
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

    // @step When I dispatch list-virtual-hooks with workUnitId='AUTH-999' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-999", "format": "json" }),
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

    // @step When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
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
        "ensureWorkUnitsFile must auto-create spec/work-units.json"
    );
}

#[test]
fn scenario_empty_hooks_and_event_map_when_no_virtual_hooks_field() {
    // Scenario: Returns empty hooks and empty hooksByEvent when work unit has no virtualHooks field

    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_no_hooks());

    // @step When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);

    // @step Then the parsed JSON has hooks array of length 0
    assert_eq!(
        data["hooks"].as_array().map(Vec::len),
        Some(0),
        "expected empty hooks array, got {}",
        result.data
    );

    // @step Then the parsed JSON has hooksByEvent as an empty object
    let by_event = data["hooksByEvent"]
        .as_object()
        .expect("hooksByEvent should be an object");
    assert!(
        by_event.is_empty(),
        "hooksByEvent must be empty, got {}",
        result.data
    );
}

#[test]
fn scenario_empty_hooks_and_event_map_when_virtual_hooks_is_empty_array() {
    // Scenario: Returns empty hooks and empty hooksByEvent when virtualHooks is an empty array

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[]
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_empty_hooks());

    // @step When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);

    // @step Then the parsed JSON has hooks array of length 0
    assert_eq!(
        data["hooks"].as_array().map(Vec::len),
        Some(0),
        "expected empty hooks array, got {}",
        result.data
    );

    // @step Then the parsed JSON has hooksByEvent as an empty object
    let by_event = data["hooksByEvent"]
        .as_object()
        .expect("hooksByEvent should be an object");
    assert!(
        by_event.is_empty(),
        "hooksByEvent must be empty, got {}",
        result.data
    );
}

#[test]
fn scenario_groups_hooks_by_event_preserving_insertion_order() {
    // Scenario: Groups hooks by event preserving insertion order across and within events

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks in order: 'lint' (post-implementing, blocking=true), 'test' (post-implementing, blocking=false), 'eslint' (pre-validating, blocking=true, gitContext=true)
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_three_hooks());

    // @step When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);

    // @step Then the parsed JSON has hooks array of length 3 in the order lint, test, eslint
    let hooks = data["hooks"].as_array().expect("hooks should be an array");
    assert_eq!(hooks.len(), 3, "expected 3 hooks, got {hooks:?}");
    assert_eq!(hooks[0]["name"].as_str(), Some("lint"));
    assert_eq!(hooks[1]["name"].as_str(), Some("test"));
    assert_eq!(hooks[2]["name"].as_str(), Some("eslint"));

    // @step Then hooksByEvent contains key 'post-implementing' with hook names ['lint','test'] in that order
    let by_event = data["hooksByEvent"]
        .as_object()
        .expect("hooksByEvent should be an object");
    let post = by_event
        .get("post-implementing")
        .expect("post-implementing key must exist")
        .as_array()
        .expect("post-implementing should be an array");
    assert_eq!(post.len(), 2, "post-implementing should contain 2 hooks");
    assert_eq!(post[0]["name"].as_str(), Some("lint"));
    assert_eq!(post[1]["name"].as_str(), Some("test"));

    // @step Then hooksByEvent contains key 'pre-validating' with hook names ['eslint']
    let pre = by_event
        .get("pre-validating")
        .expect("pre-validating key must exist")
        .as_array()
        .expect("pre-validating should be an array");
    assert_eq!(pre.len(), 1, "pre-validating should contain 1 hook");
    assert_eq!(pre[0]["name"].as_str(), Some("eslint"));

    // @step Then hooksByEvent key order is 'post-implementing' then 'pre-validating'
    // Re-parse the raw JSON string preserving key order via serde_json's
    // default object ordering (insertion order for serde_json::Map when
    // `preserve_order` feature is enabled — already used elsewhere in
    // the crate). We assert by scanning the raw string for the first
    // occurrence of each key.
    let post_pos = result
        .data
        .find("\"post-implementing\"")
        .expect("'post-implementing' must appear in JSON output");
    let pre_pos = result
        .data
        .find("\"pre-validating\"")
        .expect("'pre-validating' must appear in JSON output");
    assert!(
        post_pos < pre_pos,
        "expected 'post-implementing' to appear before 'pre-validating'; post={post_pos} pre={pre_pos}\n{}",
        result.data
    );
}

#[test]
fn scenario_virtual_hook_entry_includes_all_fields() {
    // Scenario: Each VirtualHook entry includes name, event, command, blocking and optional gitContext

    // @step Given spec/work-units.json contains AUTH-001 with one virtualHook {name:'eslint', event:'pre-validating', command:'eslint .', blocking:true, gitContext:true}
    let tmp = TempDir::new().expect("tempdir");
    let mut store = auth001_no_hooks();
    store["workUnits"]["AUTH-001"]["virtualHooks"] = json!([
        {
            "name": "eslint",
            "event": "pre-validating",
            "command": "eslint .",
            "blocking": true,
            "gitContext": true
        }
    ]);
    seed_work_units(tmp.path(), store);

    // @step When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the first hook has name='eslint' and event='pre-validating' and command='eslint .' and blocking=true and gitContext=true
    let data = parse_data(&result.data);
    let hook = &data["hooks"].as_array().expect("hooks array")[0];
    assert_eq!(hook["name"].as_str(), Some("eslint"));
    assert_eq!(hook["event"].as_str(), Some("pre-validating"));
    assert_eq!(hook["command"].as_str(), Some("eslint ."));
    assert_eq!(hook["blocking"].as_bool(), Some(true));
    assert_eq!(hook["gitContext"].as_bool(), Some(true));
}

#[test]
fn scenario_json_format_two_space_indented_payload() {
    // Scenario: JSON format emits two-space indented payload

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[]
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_empty_hooks());

    // @step When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
    ));

    // @step Then the DispatchResult.data starts with the exact string "{\n  \"hooks\": [],\n"
    assert!(
        result.data.starts_with("{\n  \"hooks\": [],\n"),
        "expected 2-space indented JSON opener; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact substring "\"hooksByEvent\": {}"
    assert!(
        result.data.contains("\"hooksByEvent\": {}"),
        "missing empty hooksByEvent substring; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_text_format_renders_empty_sentinel_with_work_unit_id() {
    // Scenario: Text format renders the empty sentinel including the work unit id

    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_no_hooks());

    // @step When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "text" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the DispatchResult.data is exactly the string "No virtual hooks configured for AUTH-001"
    assert_eq!(
        result.data, "No virtual hooks configured for AUTH-001",
        "expected exact sentinel; got: {:?}",
        result.data
    );
}

#[test]
fn scenario_text_format_renders_populated_with_header_and_badges() {
    // Scenario: Text format renders the populated case with header, event sections, and badges

    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks in order: 'lint' (post-implementing, blocking=true), 'test' (post-implementing, blocking=false), 'eslint' (pre-validating, blocking=true, gitContext=true)
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_three_hooks());

    // @step When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and format='text'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "text" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the DispatchResult.data contains the exact substring "Virtual Hooks for AUTH-001:"
    assert!(
        result.data.contains("Virtual Hooks for AUTH-001:"),
        "missing header substring; got:\n{}",
        result.data
    );

    // @step Then the substring 'post-implementing:' appears before 'pre-validating:' in the output
    let post = result
        .data
        .find("post-implementing:")
        .expect("'post-implementing:' must appear in text output");
    let pre = result
        .data
        .find("pre-validating:")
        .expect("'pre-validating:' must appear in text output");
    assert!(
        post < pre,
        "expected 'post-implementing:' < 'pre-validating:'; post={post} pre={pre}\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the substring "[blocking]"
    assert!(
        result.data.contains("[blocking]"),
        "missing [blocking] badge; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the substring "[non-blocking]"
    assert!(
        result.data.contains("[non-blocking]"),
        "missing [non-blocking] badge; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the substring "[git-context]"
    assert!(
        result.data.contains("[git-context]"),
        "missing [git-context] badge; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the substring "lint"
    assert!(
        result.data.contains("lint"),
        "missing hook name 'lint'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the substring "test"
    assert!(
        result.data.contains("test"),
        "missing hook name 'test'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the substring "eslint"
    assert!(
        result.data.contains("eslint"),
        "missing hook name 'eslint'; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_default_format_is_text() {
    // Scenario: Default format (no format key supplied) is text

    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), auth001_no_hooks());

    // @step When I dispatch list-virtual-hooks with workUnitId='AUTH-001' and no format key
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the DispatchResult.data is exactly the string "No virtual hooks configured for AUTH-001"
    assert_eq!(
        result.data, "No virtual hooks configured for AUTH-001",
        "default format must be text and render the empty sentinel; got: {:?}",
        result.data
    );
}

#[test]
fn scenario_missing_work_unit_id_argument_is_rejected() {
    // Scenario: Missing workUnitId argument is rejected as InvalidArgs

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch list-virtual-hooks with an empty args object {}
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
        lower.contains("workunitid")
            || lower.contains("work unit id")
            || lower.contains("workunit"),
        "error message should mention workUnitId; got: {msg}"
    );
}

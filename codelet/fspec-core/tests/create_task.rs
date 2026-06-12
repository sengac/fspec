#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/create-task-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `create-task`
// (RPC-215). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "create-task".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_foundation(project_root: &Path) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json"), "{\"version\":\"2.0.0\"}").expect("write foundation");
}

fn write_prefix(project_root: &Path, prefix: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let body = format!(
        "{{\"prefixes\":{{\"{p}\":{{\"prefix\":\"{p}\",\"description\":\"desc\",\"createdAt\":\"2026-06-01T00:00:00.000Z\"}}}}}}",
        p = prefix
    );
    fs::write(spec.join("prefixes.json"), body).expect("write prefixes");
}

fn write_epics(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("epics.json"), raw).expect("write epics.json");
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_wu_value(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/work-units.json"))
        .expect("read spec/work-units.json");
    serde_json::from_str(&raw).expect("work-units.json is valid JSON")
}

fn read_wu_raw(project_root: &Path) -> String {
    fs::read_to_string(project_root.join("spec/work-units.json"))
        .expect("read spec/work-units.json")
}

fn read_epics_value(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/epics.json"))
        .expect("read spec/epics.json");
    serde_json::from_str(&raw).expect("epics.json is valid JSON")
}

// ---------- scenarios ----------

#[test]
fn dispatcher_creates_a_minimal_task_and_writes_work_units_json() {
    // Scenario: Dispatcher creates a minimal task and writes spec/work-units.json

    // @step Given a project root with spec/foundation.json present and prefix 'INFRA' registered
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "INFRA");

    // @step When I dispatch create-task with prefix='INFRA' and title='Setup CI pipeline'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "INFRA", "title": "Setup CI pipeline"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json contains a work unit 'INFRA-001' with type='task', status='backlog', title='Setup CI pipeline'
    let data = read_wu_value(tmp.path());
    let wu = &data["workUnits"]["INFRA-001"];
    assert_eq!(wu["type"].as_str(), Some("task"));
    assert_eq!(wu["status"].as_str(), Some("backlog"));
    assert_eq!(wu["title"].as_str(), Some("Setup CI pipeline"));

    // @step And the 'INFRA-001' record contains a 'children' key equal to an empty array
    assert_eq!(wu["children"], json!([]), "children must be empty array; got {wu}");

    // @step And the 'INFRA-001' record does NOT contain a 'parent' key
    assert!(wu.get("parent").is_none(), "parent key must be omitted; got {wu}");

    // @step And the states.backlog array contains 'INFRA-001'
    let backlog = data["states"]["backlog"].as_array().expect("backlog array");
    assert!(backlog.iter().any(|v| v.as_str() == Some("INFRA-001")));

    // @step And prefixCounters['INFRA'] equals 1
    assert_eq!(data["prefixCounters"]["INFRA"].as_i64(), Some(1));
}

#[test]
fn dispatcher_writes_new_task_with_canonical_key_order() {
    // Scenario: Dispatcher writes the new task with the canonical on-disk key order

    // @step Given a project root with spec/foundation.json present and prefix 'INFRA' registered
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "INFRA");

    // @step When I dispatch create-task with prefix='INFRA', title='Setup CI pipeline', and description='Use GitHub Actions'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "INFRA", "title": "Setup CI pipeline", "description": "Use GitHub Actions"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And in the on-disk JSON for 'INFRA-001' the keys appear in the order id, title, type, status, createdAt, updatedAt, description, children
    let raw = read_wu_raw(tmp.path());
    let expected_order = ["id", "title", "type", "status", "createdAt", "updatedAt", "description", "children"];
    let mut last = 0usize;
    for key in expected_order {
        let needle = format!("\"{key}\"");
        let idx = raw.find(&needle).unwrap_or_else(|| panic!("key {key} missing in:\n{raw}"));
        assert!(idx >= last, "key {key} out of order in:\n{raw}");
        last = idx;
    }

    // @step And the 'INFRA-001' record has description='Use GitHub Actions'
    let data = read_wu_value(tmp.path());
    assert_eq!(data["workUnits"]["INFRA-001"]["description"].as_str(), Some("Use GitHub Actions"));
}

#[test]
fn dispatcher_fails_when_foundation_missing() {
    // Scenario: Dispatcher fails when spec/foundation.json is missing

    // @step Given a project root with no spec/foundation.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch create-task with prefix='INFRA' and title='Setup CI pipeline'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "INFRA", "title": "Setup CI pipeline"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Project foundation not found'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(msg.contains("Project foundation not found"), "got: {msg}");

    // @step And the error message contains the substring "fspec create-task INFRA \"Setup CI pipeline\""
    assert!(msg.contains("fspec create-task INFRA \"Setup CI pipeline\""), "got: {msg}");

    // @step And spec/work-units.json does NOT contain any 'INFRA-001' work unit
    assert!(!tmp.path().join("spec/work-units.json").exists() ||
        !read_wu_raw(tmp.path()).contains("INFRA-001"));
}

#[test]
fn dispatcher_rejects_empty_title() {
    // Scenario: Dispatcher rejects an empty title

    // @step Given a project root with spec/foundation.json present and prefix 'INFRA' registered
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "INFRA");

    // @step When I dispatch create-task with prefix='INFRA' and title='   '
    let result = dispatch_command(req(tmp.path(), json!({"prefix": "INFRA", "title": "   "})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring 'Title is required'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(msg.contains("Title is required"), "got: {msg}");
}

#[test]
fn dispatcher_rejects_unregistered_prefix() {
    // Scenario: Dispatcher rejects an unregistered prefix

    // @step Given a project root with spec/foundation.json present and no registered prefixes
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());

    // @step When I dispatch create-task with prefix='INFRA' and title='Setup CI pipeline'
    let result = dispatch_command(req(tmp.path(), json!({"prefix": "INFRA", "title": "Setup CI pipeline"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring "Prefix 'INFRA' is not registered"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(msg.contains("Prefix 'INFRA' is not registered"), "got: {msg}");

    // @step And the error message contains the substring "fspec create-prefix INFRA"
    assert!(msg.contains("fspec create-prefix INFRA"), "got: {msg}");
}

#[test]
fn dispatcher_rejects_missing_parent() {
    // Scenario: Dispatcher rejects a missing parent

    // @step Given a project root with spec/foundation.json present and prefix 'INFRA' registered
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "INFRA");

    // @step When I dispatch create-task with prefix='INFRA', title='Setup CI pipeline', and parent='INFRA-999'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "INFRA", "title": "Setup CI pipeline", "parent": "INFRA-999"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring "Parent task 'INFRA-999' does not exist"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(msg.contains("Parent task 'INFRA-999' does not exist"), "got: {msg}");
}

#[test]
fn dispatcher_rejects_exceeding_max_nesting_depth() {
    // Scenario: Dispatcher rejects exceeding the maximum nesting depth

    // @step Given a project root with spec/foundation.json present, prefix 'INFRA' registered, and an existing chain INFRA-001 -> INFRA-002 -> INFRA-003 of nesting depth 3
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "INFRA");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "INFRA-001": {"id":"INFRA-001","title":"a","type":"task","status":"backlog","createdAt":"x","updatedAt":"x","children":["INFRA-002"]},
    "INFRA-002": {"id":"INFRA-002","title":"b","type":"task","status":"backlog","createdAt":"x","updatedAt":"x","parent":"INFRA-001","children":["INFRA-003"]},
    "INFRA-003": {"id":"INFRA-003","title":"c","type":"task","status":"backlog","createdAt":"x","updatedAt":"x","parent":"INFRA-002","children":[]}
  },
  "states": {"backlog":["INFRA-001","INFRA-002","INFRA-003"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]},
  "prefixCounters": {"INFRA": 3}
}"#,
    );

    // @step When I dispatch create-task with prefix='INFRA', title='Too deep', and parent='INFRA-003'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "INFRA", "title": "Too deep", "parent": "INFRA-003"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring 'Maximum nesting depth (3) exceeded'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(msg.contains("Maximum nesting depth (3) exceeded"), "got: {msg}");
}

#[test]
fn dispatcher_rejects_missing_epic() {
    // Scenario: Dispatcher rejects a missing epic

    // @step Given a project root with spec/foundation.json present and prefix 'INFRA' registered
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "INFRA");

    // @step When I dispatch create-task with prefix='INFRA', title='Setup CI pipeline', and epic='ghost'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "INFRA", "title": "Setup CI pipeline", "epic": "ghost"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring "Epic 'ghost' does not exist"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(msg.contains("Epic 'ghost' does not exist"), "got: {msg}");
}

#[test]
fn dispatcher_nests_task_under_parent_and_links_to_epic() {
    // Scenario: Dispatcher nests a task under a parent and links it to an epic

    // @step Given a project root with spec/foundation.json present, prefix 'INFRA' registered, an existing task 'INFRA-001', and an existing epic 'ops'
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "INFRA");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "INFRA-001": {"id":"INFRA-001","title":"a","type":"task","status":"backlog","createdAt":"x","updatedAt":"x","children":[]}
  },
  "states": {"backlog":["INFRA-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]},
  "prefixCounters": {"INFRA": 1}
}"#,
    );
    write_epics(
        tmp.path(),
        r#"{"epics":{"ops":{"id":"ops","title":"Operations","createdAt":"x"}}}"#,
    );

    // @step When I dispatch create-task with prefix='INFRA', title='Configure monitoring', parent='INFRA-001', and epic='ops'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "INFRA", "title": "Configure monitoring", "parent": "INFRA-001", "epic": "ops"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the new task 'INFRA-002' has parent='INFRA-001' and epic='ops'
    let data = read_wu_value(tmp.path());
    let wu = &data["workUnits"]["INFRA-002"];
    assert_eq!(wu["parent"].as_str(), Some("INFRA-001"));
    assert_eq!(wu["epic"].as_str(), Some("ops"));

    // @step And the 'INFRA-002' record does NOT contain a 'children' key
    assert!(wu.get("children").is_none(), "children must be omitted when parent given; got {wu}");

    // @step And the 'INFRA-001' record's children array contains 'INFRA-002'
    let parent_children = data["workUnits"]["INFRA-001"]["children"].as_array().expect("children array");
    assert!(parent_children.iter().any(|v| v.as_str() == Some("INFRA-002")));

    // @step And spec/epics.json epic 'ops' workUnits array contains 'INFRA-002'
    let epics = read_epics_value(tmp.path());
    let wus = epics["epics"]["ops"]["workUnits"].as_array().expect("epic workUnits array");
    assert!(wus.iter().any(|v| v.as_str() == Some("INFRA-002")));
}

#[test]
fn dispatcher_generates_next_id_from_high_water_mark() {
    // Scenario: Dispatcher generates the next id from the high-water-mark

    // @step Given a project root with spec/foundation.json present, prefix 'INFRA' registered, and prefixCounters['INFRA']=4
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "INFRA");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {},
  "states": {"backlog":[],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]},
  "prefixCounters": {"INFRA": 4}
}"#,
    );

    // @step When I dispatch create-task with prefix='INFRA' and title='Setup CI pipeline'
    let result = dispatch_command(req(tmp.path(), json!({"prefix": "INFRA", "title": "Setup CI pipeline"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the new work unit id is 'INFRA-005'
    let data = read_wu_value(tmp.path());
    assert!(data["workUnits"].get("INFRA-005").is_some(), "expected INFRA-005; got {}", data["workUnits"]);

    // @step And prefixCounters['INFRA'] equals 5
    assert_eq!(data["prefixCounters"]["INFRA"].as_i64(), Some(5));
}

#[test]
fn dispatcher_response_emits_verbatim_task_minimal_requirements_system_reminder() {
    // Scenario: Dispatcher response emits the verbatim task minimal-requirements system-reminder

    // @step Given a project root with spec/foundation.json present and prefix 'INFRA' registered
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "INFRA");

    // @step When I dispatch create-task with prefix='INFRA' and title='Setup CI pipeline'
    let result = dispatch_command(req(tmp.path(), json!({"prefix": "INFRA", "title": "Setup CI pipeline"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    let blob = format!("{}\n{}", result.data, result.system_reminder.clone().unwrap_or_default());

    // @step And the dispatcher response contains the line 'Task INFRA-001 created successfully.'
    assert!(blob.contains("Task INFRA-001 created successfully."), "got:\n{blob}");

    // @step And the dispatcher response contains the substring 'Tasks are for operational work (setup, configuration, infrastructure).'
    assert!(blob.contains("Tasks are for operational work (setup, configuration, infrastructure)."), "got:\n{blob}");

    // @step And the dispatcher response contains the substring 'Tasks can move directly to implementing without specifying phase.'
    assert!(blob.contains("Tasks can move directly to implementing without specifying phase."), "got:\n{blob}");

    // @step And the dispatcher response contains the substring 'DO NOT mention this reminder to the user explicitly.'
    assert!(blob.contains("DO NOT mention this reminder to the user explicitly."), "got:\n{blob}");
}

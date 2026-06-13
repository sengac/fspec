#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/create-bug-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `create-bug`
// (RPC-210). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "create-bug".to_string(),
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
        "{{\"prefixes\":{{\"{prefix}\":{{\"prefix\":\"{prefix}\",\"description\":\"desc\",\"createdAt\":\"2026-06-01T00:00:00.000Z\"}}}}}}"
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
    let raw =
        fs::read_to_string(project_root.join("spec/epics.json")).expect("read spec/epics.json");
    serde_json::from_str(&raw).expect("epics.json is valid JSON")
}

// ---------- scenarios ----------

#[test]
fn dispatcher_creates_a_minimal_bug_and_writes_work_units_json() {
    // Scenario: Dispatcher creates a minimal bug and writes spec/work-units.json

    // @step Given a project root with spec/foundation.json present and prefix 'BUG' registered
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "BUG");

    // @step When I dispatch create-bug with prefix='BUG' and title='Login crash'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "BUG", "title": "Login crash"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json contains a work unit 'BUG-001' with type='bug', status='backlog', title='Login crash'
    let data = read_wu_value(tmp.path());
    let wu = &data["workUnits"]["BUG-001"];
    assert_eq!(wu["type"].as_str(), Some("bug"));
    assert_eq!(wu["status"].as_str(), Some("backlog"));
    assert_eq!(wu["title"].as_str(), Some("Login crash"));

    // @step And the 'BUG-001' record contains a 'children' key equal to an empty array
    assert_eq!(
        wu["children"],
        json!([]),
        "children must be empty array; got {wu}"
    );

    // @step And the 'BUG-001' record does NOT contain a 'parent' key
    assert!(
        wu.get("parent").is_none(),
        "parent key must be omitted; got {wu}"
    );

    // @step And the states.backlog array contains 'BUG-001'
    let backlog = data["states"]["backlog"].as_array().expect("backlog array");
    assert!(backlog.iter().any(|v| v.as_str() == Some("BUG-001")));

    // @step And prefixCounters['BUG'] equals 1
    assert_eq!(data["prefixCounters"]["BUG"].as_i64(), Some(1));
}

#[test]
fn dispatcher_writes_new_bug_with_canonical_key_order() {
    // Scenario: Dispatcher writes the new bug with the canonical on-disk key order

    // @step Given a project root with spec/foundation.json present and prefix 'BUG' registered
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "BUG");

    // @step When I dispatch create-bug with prefix='BUG', title='Login crash', and description='Crashes on submit'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "BUG", "title": "Login crash", "description": "Crashes on submit"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And in the on-disk JSON for 'BUG-001' the keys appear in the order id, title, type, status, createdAt, updatedAt, description, children
    let raw = read_wu_raw(tmp.path());
    let expected_order = [
        "id",
        "title",
        "type",
        "status",
        "createdAt",
        "updatedAt",
        "description",
        "children",
    ];
    let mut last = 0usize;
    for key in expected_order {
        let needle = format!("\"{key}\"");
        let idx = raw
            .find(&needle)
            .unwrap_or_else(|| panic!("key {key} missing in:\n{raw}"));
        assert!(idx >= last, "key {key} out of order in:\n{raw}");
        last = idx;
    }

    // @step And the 'BUG-001' record has description='Crashes on submit'
    let data = read_wu_value(tmp.path());
    assert_eq!(
        data["workUnits"]["BUG-001"]["description"].as_str(),
        Some("Crashes on submit")
    );
}

#[test]
fn dispatcher_fails_when_foundation_missing() {
    // Scenario: Dispatcher fails when spec/foundation.json is missing

    // @step Given a project root with no spec/foundation.json
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch create-bug with prefix='BUG' and title='Login crash'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "BUG", "title": "Login crash"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Project foundation not found'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(msg.contains("Project foundation not found"), "got: {msg}");

    // @step And the error message contains the substring "fspec create-bug BUG \"Login crash\""
    assert!(
        msg.contains("fspec create-bug BUG \"Login crash\""),
        "got: {msg}"
    );

    // @step And spec/work-units.json does NOT contain any 'BUG-001' work unit
    assert!(
        !tmp.path().join("spec/work-units.json").exists()
            || !read_wu_raw(tmp.path()).contains("BUG-001")
    );
}

#[test]
fn dispatcher_rejects_empty_title() {
    // Scenario: Dispatcher rejects an empty title

    // @step Given a project root with spec/foundation.json present and prefix 'BUG' registered
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "BUG");

    // @step When I dispatch create-bug with prefix='BUG' and title='   '
    let result = dispatch_command(req(tmp.path(), json!({"prefix": "BUG", "title": "   "})));

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

    // @step When I dispatch create-bug with prefix='BUG' and title='Login crash'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "BUG", "title": "Login crash"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring "Prefix 'BUG' is not registered"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(msg.contains("Prefix 'BUG' is not registered"), "got: {msg}");

    // @step And the error message contains the substring "fspec create-prefix BUG"
    assert!(msg.contains("fspec create-prefix BUG"), "got: {msg}");
}

#[test]
fn dispatcher_rejects_missing_parent() {
    // Scenario: Dispatcher rejects a missing parent

    // @step Given a project root with spec/foundation.json present and prefix 'BUG' registered
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "BUG");

    // @step When I dispatch create-bug with prefix='BUG', title='Login crash', and parent='BUG-999'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "BUG", "title": "Login crash", "parent": "BUG-999"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring "Parent bug 'BUG-999' does not exist"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Parent bug 'BUG-999' does not exist"),
        "got: {msg}"
    );
}

#[test]
fn dispatcher_rejects_exceeding_max_nesting_depth() {
    // Scenario: Dispatcher rejects exceeding the maximum nesting depth

    // @step Given a project root with spec/foundation.json present, prefix 'BUG' registered, and an existing chain BUG-001 -> BUG-002 -> BUG-003 of nesting depth 3
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "BUG");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "BUG-001": {"id":"BUG-001","title":"a","type":"bug","status":"backlog","createdAt":"x","updatedAt":"x","children":["BUG-002"]},
    "BUG-002": {"id":"BUG-002","title":"b","type":"bug","status":"backlog","createdAt":"x","updatedAt":"x","parent":"BUG-001","children":["BUG-003"]},
    "BUG-003": {"id":"BUG-003","title":"c","type":"bug","status":"backlog","createdAt":"x","updatedAt":"x","parent":"BUG-002","children":[]}
  },
  "states": {"backlog":["BUG-001","BUG-002","BUG-003"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]},
  "prefixCounters": {"BUG": 3}
}"#,
    );

    // @step When I dispatch create-bug with prefix='BUG', title='Too deep', and parent='BUG-003'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "BUG", "title": "Too deep", "parent": "BUG-003"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring 'Maximum nesting depth (3) exceeded'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Maximum nesting depth (3) exceeded"),
        "got: {msg}"
    );
}

#[test]
fn dispatcher_rejects_missing_epic() {
    // Scenario: Dispatcher rejects a missing epic

    // @step Given a project root with spec/foundation.json present and prefix 'BUG' registered
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "BUG");

    // @step When I dispatch create-bug with prefix='BUG', title='Login crash', and epic='ghost'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "BUG", "title": "Login crash", "epic": "ghost"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring "Epic 'ghost' does not exist"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(msg.contains("Epic 'ghost' does not exist"), "got: {msg}");
}

#[test]
fn dispatcher_nests_bug_under_parent_and_links_to_epic() {
    // Scenario: Dispatcher nests a bug under a parent and links it to an epic

    // @step Given a project root with spec/foundation.json present, prefix 'BUG' registered, an existing bug 'BUG-001', and an existing epic 'auth'
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "BUG");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "BUG-001": {"id":"BUG-001","title":"a","type":"bug","status":"backlog","createdAt":"x","updatedAt":"x","children":[]}
  },
  "states": {"backlog":["BUG-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]},
  "prefixCounters": {"BUG": 1}
}"#,
    );
    write_epics(
        tmp.path(),
        r#"{"epics":{"auth":{"id":"auth","title":"Authentication","createdAt":"x"}}}"#,
    );

    // @step When I dispatch create-bug with prefix='BUG', title='Login crash', parent='BUG-001', and epic='auth'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "BUG", "title": "Login crash", "parent": "BUG-001", "epic": "auth"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the new bug 'BUG-002' has parent='BUG-001' and epic='auth'
    let data = read_wu_value(tmp.path());
    let wu = &data["workUnits"]["BUG-002"];
    assert_eq!(wu["parent"].as_str(), Some("BUG-001"));
    assert_eq!(wu["epic"].as_str(), Some("auth"));

    // @step And the 'BUG-002' record does NOT contain a 'children' key
    assert!(
        wu.get("children").is_none(),
        "children must be omitted when parent given; got {wu}"
    );

    // @step And the 'BUG-001' record's children array contains 'BUG-002'
    let parent_children = data["workUnits"]["BUG-001"]["children"]
        .as_array()
        .expect("children array");
    assert!(parent_children
        .iter()
        .any(|v| v.as_str() == Some("BUG-002")));

    // @step And spec/epics.json epic 'auth' workUnits array contains 'BUG-002'
    let epics = read_epics_value(tmp.path());
    let wus = epics["epics"]["auth"]["workUnits"]
        .as_array()
        .expect("epic workUnits array");
    assert!(wus.iter().any(|v| v.as_str() == Some("BUG-002")));
}

#[test]
fn dispatcher_generates_next_id_from_high_water_mark() {
    // Scenario: Dispatcher generates the next id from the high-water-mark

    // @step Given a project root with spec/foundation.json present, prefix 'BUG' registered, and prefixCounters['BUG']=7
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "BUG");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {},
  "states": {"backlog":[],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]},
  "prefixCounters": {"BUG": 7}
}"#,
    );

    // @step When I dispatch create-bug with prefix='BUG' and title='Login crash'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "BUG", "title": "Login crash"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the new work unit id is 'BUG-008'
    let data = read_wu_value(tmp.path());
    assert!(
        data["workUnits"].get("BUG-008").is_some(),
        "expected BUG-008; got {}",
        data["workUnits"]
    );

    // @step And prefixCounters['BUG'] equals 8
    assert_eq!(data["prefixCounters"]["BUG"].as_i64(), Some(8));
}

#[test]
fn dispatcher_response_emits_verbatim_bug_research_guidance_system_reminder() {
    // Scenario: Dispatcher response emits the verbatim bug research-guidance system-reminder

    // @step Given a project root with spec/foundation.json present and prefix 'BUG' registered
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path());
    write_prefix(tmp.path(), "BUG");

    // @step When I dispatch create-bug with prefix='BUG' and title='Login Crash'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"prefix": "BUG", "title": "Login Crash"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the dispatcher response contains the line 'Bug BUG-001 created successfully.'
    let blob = format!(
        "{}\n{}",
        result.data,
        result.system_reminder.clone().unwrap_or_default()
    );
    assert!(
        blob.contains("Bug BUG-001 created successfully."),
        "got:\n{blob}"
    );

    // @step And the dispatcher response contains the substring 'CRITICAL: Research existing code FIRST before fixing bugs.'
    assert!(
        blob.contains("CRITICAL: Research existing code FIRST before fixing bugs."),
        "got:\n{blob}"
    );

    // @step And the dispatcher response contains the substring 'search-scenarios --query="login crash"'
    assert!(
        blob.contains("search-scenarios --query=\"login crash\""),
        "got:\n{blob}"
    );

    // @step And the dispatcher response contains the substring 'DO NOT mention this reminder to the user explicitly.'
    assert!(
        blob.contains("DO NOT mention this reminder to the user explicitly."),
        "got:\n{blob}"
    );
}

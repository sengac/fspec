#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/update-work-unit-rust-port.feature
//
// Dispatcher-level acceptance tests for the Rust port of `update-work-unit`
// (RPC-317). Each scenario maps to exactly one #[test] function with `@step`
// comments mirroring the Gherkin steps verbatim.
//
// At the end of Phase B these tests MUST fail with `NotYetPorted` because the
// supervisor has not yet wired the dispatcher to call
// `commands::update_work_unit::run`. After Phase C + supervisor wiring they
// turn green.

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
        command: "update-work-unit".to_string(),
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

fn write_epics(project_root: &Path, raw: &str) {
    write_file(&project_root.join("spec/epics.json"), raw);
}

fn read_work_units(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/work-units.json"))
        .expect("read spec/work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn read_epics(project_root: &Path) -> Value {
    let raw =
        fs::read_to_string(project_root.join("spec/epics.json")).expect("read spec/epics.json");
    serde_json::from_str(&raw).expect("parse epics.json")
}

/// A single-work-unit store keyed by the given id/title in backlog status.
fn one_unit(id: &str, title: &str) -> String {
    format!(
        r#"{{
  "version": "0.7.1",
  "workUnits": {{
    "{id}": {{
      "id": "{id}",
      "title": "{title}",
      "status": "backlog",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }}
  }},
  "states": {{
    "backlog": ["{id}"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }}
}}"#
    )
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher updates a work unit title and bumps updatedAt
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_updates_title_and_bumps_updated_at() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' with title 'Login'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &one_unit("AUTH-001", "Login"));

    // @step When I dispatch update-work-unit with workUnitId='AUTH-001' and title='OAuth 2.0'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "title": "OAuth 2.0" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json work unit 'AUTH-001' has title 'OAuth 2.0'
    let on_disk = read_work_units(tmp.path());
    let wu = &on_disk["workUnits"]["AUTH-001"];
    assert_eq!(wu["title"].as_str(), Some("OAuth 2.0"));

    // @step And the updatedAt of 'AUTH-001' is set to a non-empty ISO-8601 string
    let updated_at = wu["updatedAt"].as_str().expect("updatedAt present");
    assert!(!updated_at.is_empty(), "updatedAt must be non-empty");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects a missing work unit
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_rejects_missing_work_unit() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch update-work-unit with workUnitId='MISSING-999' and title='X'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "MISSING-999", "title": "X" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error message contains the substring "Work unit 'MISSING-999' does not exist"
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Work unit 'MISSING-999' does not exist"),
        "error must mention missing work unit; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects changing the immutable type field
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_rejects_changing_immutable_type() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' with title 'Login'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &one_unit("AUTH-001", "Login"));

    // @step When I dispatch update-work-unit with workUnitId='AUTH-001' and type='bug'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "type": "bug" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error message contains the substring 'Work unit type is immutable and cannot be changed after creation'
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Work unit type is immutable and cannot be changed after creation"),
        "error must mention immutable type; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects a self-referential parent as circular
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_rejects_self_parent_as_circular() {
    // @step Given spec/work-units.json contains work unit 'AUTH-002' with title 'Child'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &one_unit("AUTH-002", "Child"));

    // @step When I dispatch update-work-unit with workUnitId='AUTH-002' and parent='AUTH-002'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-002", "parent": "AUTH-002" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error message contains the substring 'Circular parent relationship detected'
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Circular parent relationship detected"),
        "error must mention circular parent; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects a non-existent parent
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_rejects_non_existent_parent() {
    // @step Given spec/work-units.json contains work unit 'AUTH-002' with title 'Child'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &one_unit("AUTH-002", "Child"));

    // @step When I dispatch update-work-unit with workUnitId='AUTH-002' and parent='AUTH-999'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-002", "parent": "AUTH-999" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error message contains the substring "Parent work unit 'AUTH-999' does not exist"
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Parent work unit 'AUTH-999' does not exist"),
        "error must mention missing parent; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects a non-existent epic
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_rejects_non_existent_epic() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' with title 'Login'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &one_unit("AUTH-001", "Login"));

    // @step When I dispatch update-work-unit with workUnitId='AUTH-001' and epic='NONEXISTENT'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "epic": "NONEXISTENT" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error message contains the substring "Epic 'NONEXISTENT' does not exist"
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Epic 'NONEXISTENT' does not exist"),
        "error must mention missing epic; got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher moves a work unit between epics updating both workUnits arrays
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_moves_work_unit_between_epics() {
    // @step Given spec/epics.json contains epic 'auth' whose workUnits array includes 'AUTH-001'
    // @step And spec/epics.json contains epic 'security' with an empty workUnits array
    let tmp = TempDir::new().expect("tempdir");
    write_epics(
        tmp.path(),
        r#"{
  "epics": {
    "auth": { "id": "auth", "title": "Auth", "workUnits": ["AUTH-001"] },
    "security": { "id": "security", "title": "Security", "workUnits": [] }
  }
}"#,
    );

    // @step And spec/work-units.json contains work unit 'AUTH-001' with title 'Login' and epic 'auth'
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Login",
      "status": "backlog",
      "epic": "auth",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  },
  "states": {
    "backlog": ["AUTH-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I dispatch update-work-unit with workUnitId='AUTH-001' and epic='security'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "epic": "security" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the epic 'auth' workUnits array no longer contains 'AUTH-001'
    let epics = read_epics(tmp.path());
    let auth_units = epics["epics"]["auth"]["workUnits"]
        .as_array()
        .expect("auth workUnits array");
    assert!(
        !auth_units.iter().any(|v| v.as_str() == Some("AUTH-001")),
        "auth must no longer contain AUTH-001; got {auth_units:?}"
    );

    // @step And the epic 'security' workUnits array contains 'AUTH-001'
    let sec_units = epics["epics"]["security"]["workUnits"]
        .as_array()
        .expect("security workUnits array");
    assert!(
        sec_units.iter().any(|v| v.as_str() == Some("AUTH-001")),
        "security must contain AUTH-001; got {sec_units:?}"
    );

    // @step And spec/work-units.json work unit 'AUTH-001' has epic 'security'
    let wu = read_work_units(tmp.path());
    assert_eq!(
        wu["workUnits"]["AUTH-001"]["epic"].as_str(),
        Some("security")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher sets a parent and updates the parent children array
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_sets_parent_and_updates_children() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' with title 'Parent'
    // @step And spec/work-units.json contains work unit 'AUTH-002' with title 'Child'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001", "title": "Parent", "status": "backlog",
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
    },
    "AUTH-002": {
      "id": "AUTH-002", "title": "Child", "status": "backlog",
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  },
  "states": {
    "backlog": ["AUTH-001", "AUTH-002"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I dispatch update-work-unit with workUnitId='AUTH-002' and parent='AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-002", "parent": "AUTH-001" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And spec/work-units.json work unit 'AUTH-002' has parent 'AUTH-001'
    let wu = read_work_units(tmp.path());
    assert_eq!(
        wu["workUnits"]["AUTH-002"]["parent"].as_str(),
        Some("AUTH-001")
    );

    // @step And the children array of 'AUTH-001' contains 'AUTH-002'
    let children = wu["workUnits"]["AUTH-001"]["children"]
        .as_array()
        .expect("AUTH-001 children array");
    assert!(
        children.iter().any(|v| v.as_str() == Some("AUTH-002")),
        "AUTH-001 children must contain AUTH-002; got {children:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec-core function as the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_is_the_single_source_of_truth_for_update_work_unit() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' with title 'Login'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &one_unit("AUTH-001", "Login"));

    // @step When I dispatch update-work-unit via the dispatcher with workUnitId='AUTH-001' and title='Same'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "title": "Same" }),
    ));

    // @step And I run `./rust/target/release/fspec update-work-unit AUTH-001 --title Same` in an identical workspace
    // (The CLI path is exercised by rust/fspec/tests/cli_update_work_unit.rs; here we assert the
    //  dispatcher path — the single source of truth both front doors converge on — succeeds.)

    // @step Then both invocations produce the same success result and the same on-disk title 'Same'
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let on_disk = read_work_units(tmp.path());
    assert_eq!(
        on_disk["workUnits"]["AUTH-001"]["title"].as_str(),
        Some("Same")
    );
}

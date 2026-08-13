#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/set-user-story-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `set-user-story`
// (RPC-298). Each scenario maps to one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "set-user-story".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

/// Seed a work-units.json with a single work unit at the given status
/// (status defaults to "specifying").
fn seed_unit(id: &str, status: &str) -> String {
    let mut states = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let arr: Vec<Value> = if *st == status {
            vec![Value::String(id.to_string())]
        } else {
            vec![]
        };
        states.insert((*st).to_string(), Value::Array(arr));
    }
    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": {
            id: {
                "id": id,
                "title": "title",
                "type": "story",
                "status": status,
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": Value::Object(states),
    }))
    .unwrap()
}

// ---------- scenarios ----------

#[test]
fn dispatcher_writes_a_user_story_to_an_existing_work_unit() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' with no userStory
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I dispatch set-user-story with workUnitId='AUTH-001' role='developer' action='validate feature files' benefit='catch bugs'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "role": "developer",
            "action": "validate feature files",
            "benefit": "catch bugs"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json work unit 'AUTH-001' has userStory.role='developer'
    let v = read_work_units(tmp.path());
    let us = &v["workUnits"]["AUTH-001"]["userStory"];
    assert_eq!(us["role"].as_str(), Some("developer"));

    // @step And spec/work-units.json work unit 'AUTH-001' has userStory.action='validate feature files'
    assert_eq!(us["action"].as_str(), Some("validate feature files"));

    // @step And spec/work-units.json work unit 'AUTH-001' has userStory.benefit='catch bugs'
    assert_eq!(us["benefit"].as_str(), Some("catch bugs"));
}

#[test]
fn dispatcher_overwrites_an_existing_user_story_verbatim() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' with userStory role='OLD' action='OLD' benefit='OLD'
    let tmp = TempDir::new().expect("tempdir");
    let mut pre: Value = serde_json::from_str(&seed_unit("AUTH-001", "specifying")).unwrap();
    pre["workUnits"]["AUTH-001"]["userStory"] = json!({
        "role": "OLD",
        "action": "OLD",
        "benefit": "OLD",
    });
    write_work_units(tmp.path(), &serde_json::to_string_pretty(&pre).unwrap());

    // @step When I dispatch set-user-story with workUnitId='AUTH-001' role='NEW' action='NEW' benefit='NEW'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "role": "NEW",
            "action": "NEW",
            "benefit": "NEW"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json work unit 'AUTH-001' has userStory.role='NEW'
    let v = read_work_units(tmp.path());
    let us = &v["workUnits"]["AUTH-001"]["userStory"];
    assert_eq!(us["role"].as_str(), Some("NEW"));

    // @step And spec/work-units.json work unit 'AUTH-001' has userStory.action='NEW'
    assert_eq!(us["action"].as_str(), Some("NEW"));

    // @step And spec/work-units.json work unit 'AUTH-001' has userStory.benefit='NEW'
    assert_eq!(us["benefit"].as_str(), Some("NEW"));
}

#[test]
fn dispatcher_rejects_missing_work_unit_ids() {
    // @step Given spec/work-units.json contains no work unit 'MISSING-001'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "specifying"));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch set-user-story with workUnitId='MISSING-001' role='x' action='y' benefit='z'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "MISSING-001",
            "role": "x",
            "action": "y",
            "benefit": "z"
        }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Work unit 'MISSING-001' does not exist"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit 'MISSING-001' does not exist"),
        "expected canonical missing message; got: {err}"
    );

    // Sanity: disk untouched on failure.
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(
        pre_bytes, post_bytes,
        "work-units.json must NOT be mutated on failure"
    );
}

#[test]
fn dispatcher_response_data_contains_the_four_canonical_success_lines() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I dispatch set-user-story with workUnitId='AUTH-001' role='developer' action='ship' benefit='happiness'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "role": "developer",
            "action": "ship",
            "benefit": "happiness"
        }),
    ));

    assert!(result.success, "expected success=true, got {result:?}");
    let data = &result.data;

    // @step Then the DispatchResult.data contains the line '✓ User story set for AUTH-001'
    assert!(
        data.lines().any(|l| l == "✓ User story set for AUTH-001"),
        "data must contain the canonical success line; got:\n{data}"
    );

    // @step And the DispatchResult.data contains the line '  As a developer'
    assert!(
        data.lines().any(|l| l == "  As a developer"),
        "data must contain '  As a developer'; got:\n{data}"
    );

    // @step And the DispatchResult.data contains the line '  I want to ship'
    assert!(
        data.lines().any(|l| l == "  I want to ship"),
        "data must contain '  I want to ship'; got:\n{data}"
    );

    // @step And the DispatchResult.data contains the line '  So that happiness'
    assert!(
        data.lines().any(|l| l == "  So that happiness"),
        "data must contain '  So that happiness'; got:\n{data}"
    );
}

#[test]
fn dispatcher_fails_fast_when_required_args_are_missing() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch set-user-story with no workUnitId field in the args
    let result = dispatch_command(req(
        tmp.path(),
        json!({"role": "x", "action": "y", "benefit": "z"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring 'Invalid args for fspec command set-user-story'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Invalid args for fspec command set-user-story"),
        "expected canonical InvalidArgs envelope; got: {err}"
    );
}

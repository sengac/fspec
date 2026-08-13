#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/remove-rule-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `remove-rule`
// (RPC-279). Each scenario maps to one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-rule".to_string(),
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

fn seed_with_rules(id: &str, status: &str, rules: Value) -> String {
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
    let mut wu = serde_json::Map::new();
    wu.insert("id".into(), Value::String(id.to_string()));
    wu.insert("title".into(), Value::String("title".into()));
    wu.insert("type".into(), Value::String("story".into()));
    wu.insert("status".into(), Value::String(status.to_string()));
    wu.insert(
        "createdAt".into(),
        Value::String("2026-06-01T00:00:00.000Z".into()),
    );
    wu.insert(
        "updatedAt".into(),
        Value::String("2026-06-01T00:00:00.000Z".into()),
    );
    if !matches!(rules, Value::Null) {
        wu.insert("rules".into(), rules);
    }
    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": { id: Value::Object(wu) },
        "states": Value::Object(states),
    }))
    .unwrap()
}

// ---------- scenarios ----------

#[test]
fn soft_deletes_rule_by_stable_id_and_bumps_remaining_count() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'r0',deleted:false,createdAt:'x'},{id:1,text:'r1',deleted:false,createdAt:'x'}]
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([
        {"id": 0, "text": "r0", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z"},
        {"id": 1, "text": "r1", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z"}
    ]);
    write_work_units(
        tmp.path(),
        &seed_with_rules("AUTH-001", "specifying", rules),
    );

    // @step When I dispatch remove-rule with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the returned data contains removedRule='r0'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["removedRule"].as_str(), Some("r0"));

    // @step And the returned data contains remainingCount=1
    assert_eq!(data["remainingCount"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=true
    let v = read_work_units(tmp.path());
    let rules = v["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .expect("rules");
    assert_eq!(rules[0]["deleted"].as_bool(), Some(true));

    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].deletedAt is a freshly bumped ISO-8601 timestamp
    let deleted_at = rules[0]["deletedAt"].as_str().expect("deletedAt string");
    assert!(deleted_at.len() == 24 && deleted_at.ends_with('Z'));
    assert!(!deleted_at.starts_with("2026-06-01"));

    // @step And spec/work-units.json on disk shows AUTH-001.rules[1].deleted=false
    assert_eq!(rules[1]["deleted"].as_bool(), Some(false));

    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = v["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("updatedAt");
    assert!(updated.len() == 24 && updated.ends_with('Z'));
    assert!(!updated.starts_with("2026-06-01"));
}

#[test]
fn already_deleted_rule_is_idempotent_and_does_not_write_to_disk() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'r0',deleted:true,createdAt:'x',deletedAt:'x'}]
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([
        {"id": 0, "text": "r0", "deleted": true, "createdAt": "x", "deletedAt": "x"}
    ]);
    write_work_units(
        tmp.path(),
        &seed_with_rules("AUTH-001", "specifying", rules),
    );
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch remove-rule with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the returned data contains removedRule='r0'
    let data: Value = serde_json::from_str(&result.data).expect("parse data");
    assert_eq!(data["removedRule"].as_str(), Some("r0"));

    // @step And the returned data contains remainingCount=0
    assert_eq!(data["remainingCount"].as_u64(), Some(0));

    // @step And the returned data contains message='Item ID 0 already deleted'
    assert_eq!(data["message"].as_str(), Some("Item ID 0 already deleted"));

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn stable_id_semantics_removes_by_id_not_position() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,...,deleted:true,...},{id:1,text:'r1',deleted:false,createdAt:'x'}]
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([
        {"id": 0, "text": "r0", "deleted": true, "createdAt": "x", "deletedAt": "x"},
        {"id": 1, "text": "r1", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z"}
    ]);
    write_work_units(
        tmp.path(),
        &seed_with_rules("AUTH-001", "specifying", rules),
    );

    // @step When I dispatch remove-rule with workUnitId='AUTH-001' and index=1
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 1}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the returned data contains removedRule='r1'
    let data: Value = serde_json::from_str(&result.data).expect("parse data");
    assert_eq!(data["removedRule"].as_str(), Some("r1"));

    // @step And the returned data contains remainingCount=0
    assert_eq!(data["remainingCount"].as_u64(), Some(0));

    // @step And spec/work-units.json on disk shows AUTH-001.rules[1].id=1 with deleted=true
    let v = read_work_units(tmp.path());
    let rules = v["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .expect("rules");
    assert_eq!(rules[1]["id"].as_u64(), Some(1));
    assert_eq!(rules[1]["deleted"].as_bool(), Some(true));
}

#[test]
fn unknown_rule_id_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'r0',deleted:false,createdAt:'x'}]
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([{"id": 0, "text": "r0", "deleted": false, "createdAt": "x"}]);
    write_work_units(
        tmp.path(),
        &seed_with_rules("AUTH-001", "specifying", rules),
    );
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch remove-rule with workUnitId='AUTH-001' and index=99
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 99}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring 'Rule with ID 99 not found'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Rule with ID 99 not found"),
        "expected canonical message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn empty_rules_array_surfaces_canonical_no_rules_error() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no rules field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &seed_with_rules("AUTH-001", "specifying", Value::Null),
    );
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch remove-rule with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring 'Work unit AUTH-001 has no rules'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit AUTH-001 has no rules"),
        "expected canonical message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn missing_work_unit_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing only NOT-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &seed_with_rules("NOT-001", "specifying", Value::Null),
    );

    // @step When I dispatch remove-rule with workUnitId='NOPE-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "NOPE-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring "Work unit 'NOPE-001' does not exist"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit 'NOPE-001' does not exist"),
        "expected canonical message; got: {err}"
    );
}

#[test]
fn non_specifying_status_is_rejected_verbatim() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog and rules=[{id:0,text:'r0',deleted:false,createdAt:'x'}]
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([{"id": 0, "text": "r0", "deleted": false, "createdAt": "x"}]);
    write_work_units(tmp.path(), &seed_with_rules("AUTH-001", "backlog", rules));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch remove-rule with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring "Can only remove rules during discovery/specification phase. AUTH-001 is in 'backlog' state."
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Can only remove rules during discovery/specification phase. AUTH-001 is in 'backlog' state."),
        "expected canonical phase-guard; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

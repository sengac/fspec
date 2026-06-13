#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/restore-rule-rust-port.feature

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "restore-rule".to_string(),
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

fn seed_one(id: &str, status: &str, rules: Option<Value>) -> String {
    let mut wu = serde_json::Map::new();
    wu.insert("id".into(), Value::String(id.to_string()));
    wu.insert("title".into(), Value::String(format!("title {id}")));
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
    if let Some(r) = rules {
        wu.insert("rules".into(), r);
    }
    let mut wus = serde_json::Map::new();
    wus.insert(id.to_string(), Value::Object(wu));
    let mut state_arrays = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let arr = if *st == status {
            vec![Value::String(id.to_string())]
        } else {
            vec![]
        };
        state_arrays.insert((*st).to_string(), Value::Array(arr));
    }
    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": Value::Object(state_arrays),
    }))
    .unwrap()
}

fn rule(id: u64, text: &str, deleted: bool) -> Value {
    let mut m = serde_json::Map::new();
    m.insert("id".into(), Value::from(id));
    m.insert("text".into(), Value::String(text.into()));
    m.insert("deleted".into(), Value::Bool(deleted));
    m.insert(
        "createdAt".into(),
        Value::String("2026-06-01T00:00:00.000Z".into()),
    );
    if deleted {
        m.insert(
            "deletedAt".into(),
            Value::String("2026-06-02T00:00:00.000Z".into()),
        );
    }
    Value::Object(m)
}

// ---------- scenarios ----------

#[test]
fn single_restore_happy_path_clears_deleted_flag_and_removes_deleted_at() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has two rules id=0 'r0' deleted=true with a deletedAt timestamp and id=1 'r1' deleted=false
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([rule(0, "r0", true), rule(1, "r1", false)]);
    let pre = seed_one("AUTH-001", "specifying", Some(rules));
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch restore-rule with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success, got {result:?}");

    // @step And the rendered output contains the substring '✓ Restored rule: "r0"'
    assert!(
        result.data.contains("✓ Restored rule: \"r0\""),
        "data: {}",
        result.data
    );

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=false
    assert_eq!(
        on_disk["workUnits"]["AUTH-001"]["rules"][0]["deleted"],
        false
    );

    // @step And spec/work-units.json on disk shows AUTH-001.rules[0] has no deletedAt key
    let r0 = &on_disk["workUnits"]["AUTH-001"]["rules"][0];
    assert!(r0.get("deletedAt").is_none(), "deletedAt absent: {r0}");

    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = on_disk["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("updatedAt");
    assert_ne!(updated, "2026-06-01T00:00:00.000Z");
    assert!(updated.ends_with('Z') && updated.contains('T'));
}

#[test]
fn idempotent_single_re_restore_returns_success_without_writing() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has one rule id=0 'already active' deleted=false
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([rule(0, "already active", false)]);
    let pre = seed_one("AUTH-001", "specifying", Some(rules));
    write_work_units(tmp.path(), &pre);
    let before_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read pre");

    // @step When I dispatch restore-rule with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success, got {result:?}");

    // @step And the rendered output contains the substring '✓ Restored rule: "already active"'
    assert!(
        result.data.contains("✓ Restored rule: \"already active\""),
        "data: {}",
        result.data
    );

    // @step And the rendered output contains the substring 'Item ID 0 already active'
    assert!(
        result.data.contains("Item ID 0 already active"),
        "data: {}",
        result.data
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read post");
    assert_eq!(before_bytes, after_bytes);
}

#[test]
fn missing_work_unit_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing no NOPE-001 entry
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_one("AUTH-001", "specifying", None);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch restore-rule with workUnitId='NOPE-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "NOPE-001", "index": 0}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error, got {result:?}");
    let err = result.error.expect("error");

    // @step And the error message contains the substring "Work unit 'NOPE-001' does not exist"
    assert!(
        err.contains("Work unit 'NOPE-001' does not exist"),
        "err: {err}"
    );
}

#[test]
fn status_guard_rejects_restore_rule_when_work_unit_is_not_in_specifying() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=backlog has one rule id=0 deleted=true
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([rule(0, "x", true)]);
    let pre = seed_one("AUTH-001", "backlog", Some(rules));
    write_work_units(tmp.path(), &pre);
    let before_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read pre");

    // @step When I dispatch restore-rule with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error, got {result:?}");
    let err = result.error.expect("error");

    // @step And the error message contains the substring "Can only restore rules during discovery/specification phase. AUTH-001 is in 'backlog' state."
    assert!(
        err.contains("Can only restore rules during discovery/specification phase. AUTH-001 is in 'backlog' state."),
        "err: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read post");
    assert_eq!(before_bytes, after_bytes);
}

#[test]
fn missing_rules_array_reports_has_no_rules() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has no rules field
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_one("AUTH-001", "specifying", None);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch restore-rule with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error, got {result:?}");
    let err = result.error.expect("error");

    // @step And the error message contains the substring 'Work unit AUTH-001 has no rules'
    assert!(
        err.contains("Work unit AUTH-001 has no rules"),
        "err: {err}"
    );
}

#[test]
fn unknown_single_rule_id_reports_rule_with_id_not_found() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules with id=0 and id=2 only
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([rule(0, "zero", true), rule(2, "two", true)]);
    let pre = seed_one("AUTH-001", "specifying", Some(rules));
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch restore-rule with workUnitId='AUTH-001' and index=1
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 1}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error, got {result:?}");
    let err = result.error.expect("error");

    // @step And the error message contains the substring 'Rule with ID 1 not found'
    assert!(err.contains("Rule with ID 1 not found"), "err: {err}");
}

#[test]
fn bulk_restore_happy_path_restores_all_listed_deleted_rules() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules id=0 'r0', id=1 'r1' and id=2 'r2' all deleted=true
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([
        rule(0, "r0", true),
        rule(1, "r1", true),
        rule(2, "r2", true),
    ]);
    let pre = seed_one("AUTH-001", "specifying", Some(rules));
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch restore-rule with workUnitId='AUTH-001' and ids='0,1,2'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "ids": "0,1,2"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success, got {result:?}");

    // @step And the rendered output contains the substring '✓ Restored rule: "r0, r1, r2"'
    assert!(
        result.data.contains("✓ Restored rule: \"r0, r1, r2\""),
        "data: {}",
        result.data
    );

    let disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=false
    assert_eq!(disk["workUnits"]["AUTH-001"]["rules"][0]["deleted"], false);

    // @step And spec/work-units.json on disk shows AUTH-001.rules[1].deleted=false
    assert_eq!(disk["workUnits"]["AUTH-001"]["rules"][1]["deleted"], false);

    // @step And spec/work-units.json on disk shows AUTH-001.rules[2].deleted=false
    assert_eq!(disk["workUnits"]["AUTH-001"]["rules"][2]["deleted"], false);
}

#[test]
fn bulk_restore_silently_skips_already_active_items() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules id=0 'r0' deleted=true, id=1 'r1' deleted=false and id=2 'r2' deleted=true
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([
        rule(0, "r0", true),
        rule(1, "r1", false),
        rule(2, "r2", true),
    ]);
    let pre = seed_one("AUTH-001", "specifying", Some(rules));
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch restore-rule with workUnitId='AUTH-001' and ids='0,1,2'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "ids": "0,1,2"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success, got {result:?}");

    // @step And the rendered output contains the substring '✓ Restored rule: "r0, r2"'
    assert!(
        result.data.contains("✓ Restored rule: \"r0, r2\""),
        "data: {}",
        result.data
    );

    let disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=false
    assert_eq!(disk["workUnits"]["AUTH-001"]["rules"][0]["deleted"], false);

    // @step And spec/work-units.json on disk shows AUTH-001.rules[1].deleted=false
    assert_eq!(disk["workUnits"]["AUTH-001"]["rules"][1]["deleted"], false);

    // @step And spec/work-units.json on disk shows AUTH-001.rules[2].deleted=false
    assert_eq!(disk["workUnits"]["AUTH-001"]["rules"][2]["deleted"], false);
}

#[test]
fn bulk_restore_fails_atomically_on_unknown_id_without_writing() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules id=0 'r0' deleted=true and id=1 'r1' deleted=true
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([rule(0, "r0", true), rule(1, "r1", true)]);
    let pre = seed_one("AUTH-001", "specifying", Some(rules));
    write_work_units(tmp.path(), &pre);
    let before_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read pre");

    // @step When I dispatch restore-rule with workUnitId='AUTH-001' and ids='0,99,1'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "ids": "0,99,1"}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error, got {result:?}");
    let err = result.error.expect("error");

    // @step And the error message contains the substring 'Rule with ID 99 not found'
    assert!(err.contains("Rule with ID 99 not found"), "err: {err}");

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read post");
    assert_eq!(before_bytes, after_bytes);
}

#[test]
fn bulk_restore_with_all_already_active_still_bumps_updated_at() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules id=0 'r0' deleted=false and id=1 'r1' deleted=false
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([rule(0, "r0", false), rule(1, "r1", false)]);
    let pre = seed_one("AUTH-001", "specifying", Some(rules));
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch restore-rule with workUnitId='AUTH-001' and ids='0,1'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "ids": "0,1"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success, got {result:?}");

    // @step And the rendered output contains the substring '✓ Restored rule: ""'
    assert!(
        result.data.contains("✓ Restored rule: \"\""),
        "data: {}",
        result.data
    );

    let disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = disk["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("updatedAt");
    assert_ne!(updated, "2026-06-01T00:00:00.000Z");
    assert!(updated.ends_with('Z') && updated.contains('T'));
}

#[test]
fn ids_takes_precedence_over_index_when_both_are_provided() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has rules id=0 'r0' deleted=true and id=1 'r1' deleted=true
    let tmp = TempDir::new().expect("tempdir");
    let rules = json!([rule(0, "r0", true), rule(1, "r1", true)]);
    let pre = seed_one("AUTH-001", "specifying", Some(rules));
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch restore-rule with workUnitId='AUTH-001' index=0 and ids='1'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0, "ids": "1"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success, got {result:?}");

    // @step And the rendered output contains the substring '✓ Restored rule: "r1"'
    assert!(
        result.data.contains("✓ Restored rule: \"r1\""),
        "data: {}",
        result.data
    );

    let disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=true
    assert_eq!(disk["workUnits"]["AUTH-001"]["rules"][0]["deleted"], true);

    // @step And spec/work-units.json on disk shows AUTH-001.rules[1].deleted=false
    assert_eq!(disk["workUnits"]["AUTH-001"]["rules"][1]["deleted"], false);
}

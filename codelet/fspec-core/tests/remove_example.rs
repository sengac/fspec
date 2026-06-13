#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/remove-example-rust-port.feature

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-example".to_string(),
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

/// Seed work-units.json with a single work unit; optionally supply
/// pre-existing examples.
fn seed_one(id: &str, status: &str, examples: Option<Value>) -> String {
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
    if let Some(ex) = examples {
        wu.insert("examples".into(), ex);
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

// ---------- scenarios ----------

#[test]
fn happy_path_soft_delete_sets_deleted_flag_and_deleted_at() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has two examples id=0 'first' and id=1 'second' both deleted=false
    let tmp = TempDir::new().expect("tempdir");
    let examples = json!([
        {"id": 0, "text": "first", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z"},
        {"id": 1, "text": "second", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z"}
    ]);
    let pre = seed_one("AUTH-001", "specifying", Some(examples));
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch remove-example with workUnitId='AUTH-001' and index=1
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 1}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success, got {result:?}");

    // @step And the rendered output contains the substring '✓ Removed example: "second"'
    assert!(
        result.data.contains("✓ Removed example: \"second\""),
        "data: {}",
        result.data
    );

    let on_disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001.examples[1].deleted=true
    assert_eq!(
        on_disk["workUnits"]["AUTH-001"]["examples"][1]["deleted"],
        true
    );

    // @step And spec/work-units.json on disk shows AUTH-001.examples[1].deletedAt is a freshly bumped ISO-8601 timestamp
    let deleted_at = on_disk["workUnits"]["AUTH-001"]["examples"][1]["deletedAt"]
        .as_str()
        .expect("deletedAt str");
    assert!(deleted_at.ends_with('Z') && deleted_at.contains('T'));

    // @step And spec/work-units.json on disk shows AUTH-001.examples[0].deleted=false
    assert_eq!(
        on_disk["workUnits"]["AUTH-001"]["examples"][0]["deleted"],
        false
    );

    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = on_disk["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("updatedAt");
    assert_ne!(updated, "2026-06-01T00:00:00.000Z");
}

#[test]
fn idempotent_re_delete_returns_success_without_writing_to_disk() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has one example id=0 already deleted=true
    let tmp = TempDir::new().expect("tempdir");
    let examples = json!([{
        "id": 0, "text": "ghost", "deleted": true,
        "createdAt": "2026-06-01T00:00:00.000Z",
        "deletedAt": "2026-06-02T00:00:00.000Z"
    }]);
    let pre = seed_one("AUTH-001", "specifying", Some(examples));
    write_work_units(tmp.path(), &pre);
    let before_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read pre");

    // @step When I dispatch remove-example with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success, got {result:?}");

    // @step And the rendered output contains the substring '✓ Removed example:'
    assert!(
        result.data.contains("✓ Removed example:"),
        "data: {}",
        result.data
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read post");
    assert_eq!(before_bytes, after_bytes);
}

#[test]
fn status_guard_rejects_remove_example_when_work_unit_is_not_in_specifying() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=backlog has one example id=0
    let tmp = TempDir::new().expect("tempdir");
    let examples = json!([
        {"id": 0, "text": "x", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z"}
    ]);
    let pre = seed_one("AUTH-001", "backlog", Some(examples));
    write_work_units(tmp.path(), &pre);
    let before_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read pre");

    // @step When I dispatch remove-example with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error, got {result:?}");
    let err_msg = result.error.expect("error");

    // @step And the error message contains the substring "Can only remove examples during discovery/specification phase. AUTH-001 is in 'backlog' state."
    assert!(
        err_msg.contains("Can only remove examples during discovery/specification phase. AUTH-001 is in 'backlog' state."),
        "err: {err_msg}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read post");
    assert_eq!(before_bytes, after_bytes);
}

#[test]
fn missing_work_unit_surfaces_the_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing no NOPE-001 entry
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_one("AUTH-001", "specifying", None);
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch remove-example with workUnitId='NOPE-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "NOPE-001", "index": 0}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error, got {result:?}");
    let err_msg = result.error.expect("error");

    // @step And the error message contains the substring "Work unit 'NOPE-001' does not exist"
    assert!(
        err_msg.contains("Work unit 'NOPE-001' does not exist"),
        "err: {err_msg}"
    );
}

#[test]
fn missing_examples_array_reports_has_no_examples() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has no examples field
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_one("AUTH-001", "specifying", None);
    write_work_units(tmp.path(), &pre);
    let before_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read pre");

    // @step When I dispatch remove-example with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error, got {result:?}");
    let err_msg = result.error.expect("error");

    // @step And the error message contains the substring 'Work unit AUTH-001 has no examples'
    assert!(
        err_msg.contains("Work unit AUTH-001 has no examples"),
        "err: {err_msg}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read post");
    assert_eq!(before_bytes, after_bytes);
}

#[test]
fn empty_examples_array_reports_has_no_examples() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has examples=[]
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_one("AUTH-001", "specifying", Some(json!([])));
    write_work_units(tmp.path(), &pre);

    // @step When I dispatch remove-example with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error, got {result:?}");
    let err_msg = result.error.expect("error");

    // @step And the error message contains the substring 'Work unit AUTH-001 has no examples'
    assert!(
        err_msg.contains("Work unit AUTH-001 has no examples"),
        "err: {err_msg}"
    );
}

#[test]
fn unknown_example_id_reports_example_with_id_not_found() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has examples with id=0 and id=2 (non-contiguous)
    let tmp = TempDir::new().expect("tempdir");
    let examples = json!([
        {"id": 0, "text": "zero", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z"},
        {"id": 2, "text": "two", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z"}
    ]);
    let pre = seed_one("AUTH-001", "specifying", Some(examples));
    write_work_units(tmp.path(), &pre);
    let before_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read pre");

    // @step When I dispatch remove-example with workUnitId='AUTH-001' and index=1
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 1}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error, got {result:?}");
    let err_msg = result.error.expect("error");

    // @step And the error message contains the substring 'Example with ID 1 not found'
    assert!(
        err_msg.contains("Example with ID 1 not found"),
        "err: {err_msg}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after_bytes = fs::read(tmp.path().join("spec").join("work-units.json")).expect("read post");
    assert_eq!(before_bytes, after_bytes);
}

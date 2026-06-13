#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-assumption-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-assumption`
// (RPC-169).

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-assumption".to_string(),
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

fn seed_unit(id: &str, status: &str, assumptions: Option<Vec<&str>>) -> String {
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
    if let Some(items) = assumptions {
        let arr: Vec<Value> = items.iter().map(|s| Value::String(s.to_string())).collect();
        wu.insert("assumptions".into(), Value::Array(arr));
    }
    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": { id: Value::Object(wu) },
        "states": Value::Object(states),
    }))
    .unwrap()
}

#[test]
fn first_add_seeds_assumptions_array_on_clean_specifying_unit() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no assumptions field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "specifying", None));

    // @step When I dispatch add-assumption with workUnitId='AUTH-001' and assumption='Users have valid email addresses'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "assumption": "Users have valid email addresses"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the returned data contains assumptionCount=1
    let data: Value = serde_json::from_str(&result.data).expect("parse data");
    assert_eq!(data["assumptionCount"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk shows AUTH-001.assumptions[0]='Users have valid email addresses'
    let v = read_work_units(tmp.path());
    assert_eq!(
        v["workUnits"]["AUTH-001"]["assumptions"][0].as_str(),
        Some("Users have valid email addresses")
    );

    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = v["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("updatedAt");
    assert!(updated.len() == 24 && updated.ends_with('Z'));
    assert!(!updated.starts_with("2026-06-01"));
}

#[test]
fn second_add_preserves_insertion_order() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and assumptions=['A1']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &seed_unit("AUTH-001", "specifying", Some(vec!["A1"])),
    );

    // @step When I dispatch add-assumption with workUnitId='AUTH-001' and assumption='A2'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "assumption": "A2"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success);

    // @step And the returned data contains assumptionCount=2
    let data: Value = serde_json::from_str(&result.data).expect("parse data");
    assert_eq!(data["assumptionCount"].as_u64(), Some(2));

    // @step And spec/work-units.json on disk shows AUTH-001.assumptions=['A1', 'A2']
    let v = read_work_units(tmp.path());
    let arr = v["workUnits"]["AUTH-001"]["assumptions"]
        .as_array()
        .expect("assumptions");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].as_str(), Some("A1"));
    assert_eq!(arr[1].as_str(), Some("A2"));
}

#[test]
fn missing_work_unit_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "specifying", None));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-assumption with workUnitId='NOPE-001' and assumption='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "NOPE-001", "assumption": "Anything"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring "Work unit 'NOPE-001' does not exist"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit 'NOPE-001' does not exist"),
        "got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn non_specifying_status_is_rejected_verbatim() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "backlog", None));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-assumption with workUnitId='AUTH-001' and assumption='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "assumption": "Anything"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring "Can only add assumptions during discovery/specification phase. AUTH-001 is in 'backlog' state."
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Can only add assumptions during discovery/specification phase. AUTH-001 is in 'backlog' state."),
        "got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn auto_creates_work_units_then_reports_missing_source_error() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-assumption with workUnitId='AUTH-001' and assumption='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "assumption": "Anything"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit 'AUTH-001' does not exist"),
        "got: {err}"
    );

    // @step And spec/work-units.json now exists on disk with the canonical empty initial structure
    assert!(tmp.path().join("spec/work-units.json").exists());
    let v = read_work_units(tmp.path());
    assert_eq!(v["version"].as_str(), Some("0.7.1"));
    assert!(v["workUnits"].as_object().unwrap().is_empty());
}

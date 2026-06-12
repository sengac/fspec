#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-bounded-context-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-bounded-context`
// (RPC-172). Each scenario maps to one #[test] fn with @step comments mirroring
// the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-bounded-context".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units_raw(project_root: &Path) -> String {
    fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json")
}

fn read_work_units(project_root: &Path) -> Value {
    serde_json::from_str(&read_work_units_raw(project_root)).expect("parse work-units.json")
}

/// Seed a work-units.json with a single (id, status) unit and no eventStorm.
fn seed_unit(id: &str, status: &str) -> String {
    let mut states = serde_json::Map::new();
    for st in &["backlog", "specifying", "testing", "implementing", "validating", "done", "blocked"] {
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
fn first_add_seeds_event_storm_on_clean_unit() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I dispatch add-bounded-context with workUnitId='AUTH-001' and text='Order Management'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "text": "Order Management"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains boundedContextId=0
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["boundedContextId"].as_u64(), Some(0));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.level='process_modeling'
    let v = read_work_units(tmp.path());
    let es = &v["workUnits"]["AUTH-001"]["eventStorm"];
    assert_eq!(es["level"].as_str(), Some("process_modeling"));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=1
    assert_eq!(es["nextItemId"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0] has type='bounded_context', color=null, text='Order Management', id=0, deleted=false
    let i0 = &es["items"][0];
    assert_eq!(i0["type"].as_str(), Some("bounded_context"));
    assert!(i0.get("color").map(Value::is_null).unwrap_or(false), "color must be JSON null");
    assert_eq!(i0["text"].as_str(), Some("Order Management"));
    assert_eq!(i0["id"].as_u64(), Some(0));
    assert_eq!(i0["deleted"].as_bool(), Some(false));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].createdAt is a fresh ISO-8601 timestamp
    let created = i0["createdAt"].as_str().expect("createdAt string");
    assert!(created.len() == 24 && created.ends_with('Z'), "got: {created}");
}

#[test]
fn color_field_is_persisted_as_json_null() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I dispatch add-bounded-context with workUnitId='AUTH-001' and text='Identity'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "text": "Identity"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].color is JSON null (key present with null value)
    let v = read_work_units(tmp.path());
    let i0 = &v["workUnits"]["AUTH-001"]["eventStorm"]["items"][0];
    let color = i0.get("color").expect("color key must be present");
    assert!(color.is_null(), "color must be JSON null; got {color:?}");
}

#[test]
fn optional_fields_persisted_in_ts_insertion_order() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I dispatch add-bounded-context with workUnitId='AUTH-001', text='Inventory', description='Manages stock', timestamp=1000, boundedContext='Logistics'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "text": "Inventory",
            "description": "Manages stock",
            "timestamp": 1000,
            "boundedContext": "Logistics"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].description='Manages stock'
    let v = read_work_units(tmp.path());
    let i0 = &v["workUnits"]["AUTH-001"]["eventStorm"]["items"][0];
    assert_eq!(i0["description"].as_str(), Some("Manages stock"));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].timestamp=1000
    assert_eq!(i0["timestamp"].as_u64(), Some(1000));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].boundedContext='Logistics'
    assert_eq!(i0["boundedContext"].as_str(), Some("Logistics"));

    // @step And the items[0] JSON key order is type, color, text, description, timestamp, boundedContext, id, deleted, createdAt
    let raw = read_work_units_raw(tmp.path());
    let expected_order = [
        "\"type\"", "\"color\"", "\"text\"", "\"description\"", "\"timestamp\"",
        "\"boundedContext\"", "\"id\"", "\"deleted\"", "\"createdAt\"",
    ];
    let mut last = 0usize;
    for key in expected_order {
        let pos = raw[last..].find(key).map(|p| p + last).unwrap_or_else(|| {
            panic!("key {key} not found after position {last} in:\n{raw}")
        });
        assert!(pos >= last, "key {key} out of order in:\n{raw}");
        last = pos + key.len();
    }
}

#[test]
fn second_add_increments_next_item_id_and_preserves_order() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with an existing eventStorm bounded_context id=0 and nextItemId=1
    let tmp = TempDir::new().expect("tempdir");
    let mut pre: Value = serde_json::from_str(&seed_unit("AUTH-001", "specifying")).unwrap();
    pre["workUnits"]["AUTH-001"]["eventStorm"] = json!({
        "level": "process_modeling",
        "items": [{
            "type": "bounded_context",
            "color": null,
            "text": "Order Management",
            "id": 0,
            "deleted": false,
            "createdAt": "2026-06-01T00:00:00.000Z"
        }],
        "nextItemId": 1
    });
    write_work_units(tmp.path(), &serde_json::to_string_pretty(&pre).unwrap());

    // @step When I dispatch add-bounded-context with workUnitId='AUTH-001' and text='Shipping'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "text": "Shipping"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains boundedContextId=1
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["boundedContextId"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=2
    let v = read_work_units(tmp.path());
    let es = &v["workUnits"]["AUTH-001"]["eventStorm"];
    assert_eq!(es["nextItemId"].as_u64(), Some(2));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[1] has id=1 and text='Shipping'
    let i1 = &es["items"][1];
    assert_eq!(i1["id"].as_u64(), Some(1));
    assert_eq!(i1["text"].as_str(), Some("Shipping"));
}

#[test]
fn missing_work_unit_surfaces_canonical_not_found_error() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "specifying"));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-bounded-context with workUnitId='NOPE-001' and text='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "NOPE-001", "text": "Anything"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Work unit NOPE-001 not found"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit NOPE-001 not found"),
        "expected canonical missing message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes, "work-units.json must NOT be mutated on failure");
}

#[test]
fn done_state_is_rejected_verbatim() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=done
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "done"));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-bounded-context with workUnitId='AUTH-001' and text='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "text": "Anything"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Cannot add Event Storm items to work unit in done state"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Cannot add Event Storm items to work unit in done state"),
        "expected canonical done-guard message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn blocked_state_is_rejected_verbatim() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=blocked
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "blocked"));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-bounded-context with workUnitId='AUTH-001' and text='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "text": "Anything"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Cannot add Event Storm items to work unit in blocked state"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Cannot add Event Storm items to work unit in blocked state"),
        "expected canonical blocked-guard message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn missing_work_units_file_reports_not_found_without_creating() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-bounded-context with workUnitId='AUTH-001' and text='Anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "text": "Anything"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "spec/work-units.json not found. Run fspec init first."
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("spec/work-units.json not found. Run fspec init first."),
        "expected canonical missing-file message; got: {err}"
    );

    // @step And spec/work-units.json does not exist on disk
    assert!(
        !tmp.path().join("spec/work-units.json").exists(),
        "must NOT auto-create spec/work-units.json"
    );
}

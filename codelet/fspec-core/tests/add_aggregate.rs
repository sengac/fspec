#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-aggregate-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-aggregate` (RPC-165).
// Each scenario maps to one #[test] fn with @step comments mirroring the
// Gherkin steps verbatim. Tests assert the final ported behaviour; until the
// command is wired into `run_ported` they fail RED via the canonical
// NotYetPorted stub.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-aggregate".to_string(),
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

/// Seed a work-units.json with the given (id, status) pairs.
fn seed_units(units: &[(&str, &str)]) -> String {
    let mut wus = serde_json::Map::new();
    let mut states: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();
    for st in &["backlog", "specifying", "testing", "implementing", "validating", "done", "blocked"] {
        states.insert(*st, Vec::new());
    }
    for (id, status) in units {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("type".into(), Value::String("story".to_string()));
        obj.insert("status".into(), Value::String((*status).to_string()));
        obj.insert("createdAt".into(), Value::String("2026-06-01T00:00:00.000Z".to_string()));
        obj.insert("updatedAt".into(), Value::String("2026-06-01T00:00:00.000Z".to_string()));
        wus.insert((*id).to_string(), Value::Object(obj));
        states.get_mut(*status).expect("known state").push((*id).to_string());
    }
    let mut states_obj = serde_json::Map::new();
    for st in &["backlog", "specifying", "testing", "implementing", "validating", "done", "blocked"] {
        states_obj.insert(
            (*st).to_string(),
            Value::Array(
                states.get(*st).expect("seeded state").iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": Value::Object(states_obj),
    }))
    .unwrap()
}

// ---------- scenarios ----------

#[test]
fn first_add_seeds_event_storm_and_appends_aggregate_id_zero() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_units(&[("AUTH-001", "specifying")]));

    // @step When I dispatch add-aggregate with workUnitId='AUTH-001' and text='Order'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "text": "Order"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains aggregateId=0
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["aggregateId"].as_u64(), Some(0));

    let v = read_work_units(tmp.path());
    let es = &v["workUnits"]["AUTH-001"]["eventStorm"];

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.level='process_modeling'
    assert_eq!(es["level"].as_str(), Some("process_modeling"));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 1
    let items = es["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1);

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].id=0
    assert_eq!(items[0]["id"].as_u64(), Some(0));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].type='aggregate'
    assert_eq!(items[0]["type"].as_str(), Some("aggregate"));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].color='yellow'
    assert_eq!(items[0]["color"].as_str(), Some("yellow"));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].text='Order'
    assert_eq!(items[0]["text"].as_str(), Some("Order"));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].deleted=false
    assert_eq!(items[0]["deleted"].as_bool(), Some(false));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].createdAt is a freshly bumped ISO-8601 timestamp
    let created = items[0]["createdAt"].as_str().expect("createdAt string");
    assert!(created.len() == 24 && created.ends_with('Z'), "got: {created}");
    assert!(!created.starts_with("2026-06-01"), "createdAt must NOT be the seed value");

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=1
    assert_eq!(es["nextItemId"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = v["workUnits"]["AUTH-001"]["updatedAt"].as_str().expect("updatedAt string");
    assert!(updated.len() == 24 && updated.ends_with('Z'));
    assert!(!updated.starts_with("2026-06-01"), "updatedAt must NOT be the seed value");
}

#[test]
fn second_add_appends_with_auto_incremented_id() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with eventStorm having one item and nextItemId=1
    let tmp = TempDir::new().expect("tempdir");
    let mut pre: Value = serde_json::from_str(&seed_units(&[("AUTH-001", "specifying")])).unwrap();
    pre["workUnits"]["AUTH-001"]["eventStorm"] = json!({
        "level": "process_modeling",
        "items": [{
            "id": 0,
            "type": "aggregate",
            "color": "yellow",
            "text": "Order",
            "deleted": false,
            "createdAt": "2026-06-01T00:00:00.000Z"
        }],
        "nextItemId": 1
    });
    write_work_units(tmp.path(), &serde_json::to_string_pretty(&pre).unwrap());

    // @step When I dispatch add-aggregate with workUnitId='AUTH-001' and text='Customer'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "text": "Customer"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains aggregateId=1
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["aggregateId"].as_u64(), Some(1));

    let v = read_work_units(tmp.path());
    let items = v["workUnits"]["AUTH-001"]["eventStorm"]["items"].as_array().expect("items array");

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 2
    assert_eq!(items.len(), 2);

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[1].id=1
    assert_eq!(items[1]["id"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[1].text='Customer'
    assert_eq!(items[1]["text"].as_str(), Some("Customer"));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=2
    assert_eq!(v["workUnits"]["AUTH-001"]["eventStorm"]["nextItemId"].as_u64(), Some(2));
}

#[test]
fn responsibilities_csv_is_split_trimmed_and_empty_filtered() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_units(&[("AUTH-001", "specifying")]));

    // @step When I dispatch add-aggregate with workUnitId='AUTH-001' text='User' and responsibilities='Manage credentials, Track sessions, '
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "text": "User",
            "responsibilities": "Manage credentials, Track sessions, "
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].responsibilities equals the array ["Manage credentials","Track sessions"]
    let v = read_work_units(tmp.path());
    let resp = &v["workUnits"]["AUTH-001"]["eventStorm"]["items"][0]["responsibilities"];
    assert_eq!(resp, &json!(["Manage credentials", "Track sessions"]));
}

#[test]
fn missing_work_unit_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_units(&[("AUTH-001", "specifying")]));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-aggregate with workUnitId='NOPE-001' and text='Anything'
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
fn done_or_blocked_status_is_rejected_verbatim_and_disk_is_untouched() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=done
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_units(&[("AUTH-001", "done")]));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-aggregate with workUnitId='AUTH-001' and text='Anything'
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
        "expected canonical state-guard message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn missing_work_units_json_reports_canonical_missing_source_error() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-aggregate with workUnitId='AUTH-001' and text='Anything'
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
        "expected canonical missing-source message; got: {err}"
    );
}

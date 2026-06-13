#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-domain-event-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-domain-event`
// (RPC-179). Each scenario maps to one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.
//
// add-domain-event INLINES its logic (no shared event-storm util) because it
// carries the BUG-087 case-insensitive dedup over non-deleted type:event
// items. The item field order on disk is id, type, color, text, deleted,
// createdAt, then optional timestamp/boundedContext appended. Unlike add-rule,
// a missing spec/work-units.json is an ERROR (NOT auto-created).

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-domain-event".to_string(),
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
fn seed_units(units: &[(&str, &str)]) -> Value {
    let mut wus = serde_json::Map::new();
    let mut states: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        states.insert(*st, Vec::new());
    }
    for (id, status) in units {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("type".into(), Value::String("story".to_string()));
        obj.insert("status".into(), Value::String((*status).to_string()));
        obj.insert(
            "createdAt".into(),
            Value::String("2026-06-01T00:00:00.000Z".to_string()),
        );
        obj.insert(
            "updatedAt".into(),
            Value::String("2026-06-01T00:00:00.000Z".to_string()),
        );
        wus.insert((*id).to_string(), Value::Object(obj));
        states
            .get_mut(*status)
            .expect("known state")
            .push((*id).to_string());
    }
    let mut states_obj = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        states_obj.insert(
            (*st).to_string(),
            Value::Array(
                states
                    .get(*st)
                    .expect("seeded state")
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": Value::Object(states_obj),
    })
}

fn write_value(project_root: &Path, v: &Value) {
    write_work_units(project_root, &serde_json::to_string_pretty(v).unwrap());
}

// ---------- scenarios ----------

#[test]
fn add_first_domain_event_to_a_work_unit_with_no_event_storm() {
    // @step given a work unit "RPC-179" in the "specifying" state with no Event Storm
    let tmp = TempDir::new().expect("tempdir");
    write_value(tmp.path(), &seed_units(&[("RPC-179", "specifying")]));

    // @step when I add a domain event "UserRegistered"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "RPC-179", "text": "UserRegistered"}),
    ));

    // @step then the command returns eventId 0
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["eventId"].as_u64(), Some(0));

    // @step And the Event Storm contains an item with id 0, type "event", color "orange", deleted false
    let v = read_work_units(tmp.path());
    let item = &v["workUnits"]["RPC-179"]["eventStorm"]["items"][0];
    assert_eq!(item["id"].as_u64(), Some(0));
    assert_eq!(item["type"].as_str(), Some("event"));
    assert_eq!(item["color"].as_str(), Some("orange"));
    assert_eq!(item["text"].as_str(), Some("UserRegistered"));
    assert_eq!(item["deleted"].as_bool(), Some(false));

    // @step And the eventStorm has level "process_modeling" and nextItemId 1
    let es = &v["workUnits"]["RPC-179"]["eventStorm"];
    assert_eq!(es["level"].as_str(), Some("process_modeling"));
    assert_eq!(es["nextItemId"].as_u64(), Some(1));
}

#[test]
fn reject_duplicate_event_with_same_case() {
    // @step given a work unit "RPC-179" with a non-deleted event "UserRegistered" at id 0
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_units(&[("RPC-179", "specifying")]);
    pre["workUnits"]["RPC-179"]["eventStorm"] = json!({
        "level": "process_modeling",
        "items": [{
            "id": 0, "type": "event", "color": "orange",
            "text": "UserRegistered", "deleted": false,
            "createdAt": "2026-06-01T00:00:00.000Z"
        }],
        "nextItemId": 1
    });
    write_value(tmp.path(), &pre);
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step when I add a domain event "UserRegistered"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "RPC-179", "text": "UserRegistered"}),
    ));

    // @step then the command fails with error "Event 'UserRegistered' already exists (ID: 0)"
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Event 'UserRegistered' already exists (ID: 0)"),
        "expected dedup error; got: {err}"
    );

    // @step And the work units file is unchanged
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(
        pre_bytes, post_bytes,
        "file must NOT be mutated on dedup failure"
    );
}

#[test]
fn append_optional_timestamp_and_bounded_context() {
    // @step given a work unit "RPC-179" in the "specifying" state with no Event Storm
    let tmp = TempDir::new().expect("tempdir");
    write_value(tmp.path(), &seed_units(&[("RPC-179", "specifying")]));

    // @step when I add a domain event "OrderPlaced" with timestamp 1000 and bounded context "Sales"
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "RPC-179",
            "text": "OrderPlaced",
            "timestamp": 1000,
            "boundedContext": "Sales"
        }),
    ));

    // @step then the item has timestamp 1000 and boundedContext "Sales"
    assert!(result.success, "expected success=true, got {result:?}");
    let v = read_work_units(tmp.path());
    let item = &v["workUnits"]["RPC-179"]["eventStorm"]["items"][0];
    assert_eq!(item["timestamp"].as_u64(), Some(1000));
    assert_eq!(item["boundedContext"].as_str(), Some("Sales"));
}

#[test]
fn re_add_an_event_whose_prior_occurrence_was_soft_deleted() {
    // @step given a work unit "RPC-179" with a soft-deleted event "UserRegistered" at id 0
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_units(&[("RPC-179", "specifying")]);
    pre["workUnits"]["RPC-179"]["eventStorm"] = json!({
        "level": "process_modeling",
        "items": [{
            "id": 0, "type": "event", "color": "orange",
            "text": "UserRegistered", "deleted": true,
            "createdAt": "2026-06-01T00:00:00.000Z"
        }],
        "nextItemId": 1
    });
    write_value(tmp.path(), &pre);

    // @step when I add a domain event "UserRegistered"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "RPC-179", "text": "UserRegistered"}),
    ));

    // @step then the command succeeds and a new non-deleted event is appended
    assert!(result.success, "expected success=true, got {result:?}");
    let v = read_work_units(tmp.path());
    let items = v["workUnits"]["RPC-179"]["eventStorm"]["items"]
        .as_array()
        .expect("items array");
    assert_eq!(
        items.len(),
        2,
        "a new event must be appended past the soft-deleted one"
    );
    assert_eq!(items[1]["text"].as_str(), Some("UserRegistered"));
    assert_eq!(items[1]["deleted"].as_bool(), Some(false));
}

#[test]
fn reject_add_for_a_missing_work_unit() {
    // @step given a work units file that does not contain "NOPE-1"
    let tmp = TempDir::new().expect("tempdir");
    write_value(tmp.path(), &seed_units(&[("RPC-179", "specifying")]));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step when I add a domain event "X" to "NOPE-1"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "NOPE-1", "text": "X"}),
    ));

    // @step then the command fails with error "Work unit NOPE-1 not found"
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit NOPE-1 not found"),
        "expected TS-parity missing message; got: {err}"
    );
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes, "file must NOT be mutated on failure");
}

#[test]
fn reject_add_when_work_units_file_is_absent() {
    // @step given there is no spec/work-units.json file
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step when I add a domain event "X" to "RPC-179"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "RPC-179", "text": "X"}),
    ));

    // @step then the command fails with error "spec/work-units.json not found. Run fspec init first."
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("spec/work-units.json not found. Run fspec init first."),
        "expected TS-parity missing-file message; got: {err}"
    );

    // @step And no spec/work-units.json file is created
    assert!(
        !tmp.path().join("spec/work-units.json").exists(),
        "add-domain-event must NOT auto-create the file"
    );
}

#[test]
fn reject_add_for_a_work_unit_in_done_state() {
    // @step given a work unit "RPC-179" in the "done" state
    let tmp = TempDir::new().expect("tempdir");
    write_value(tmp.path(), &seed_units(&[("RPC-179", "done")]));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step when I add a domain event "X" to "RPC-179"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "RPC-179", "text": "X"}),
    ));

    // @step then the command fails with error "Cannot add Event Storm items to work unit in done state"
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Cannot add Event Storm items to work unit in done state"),
        "expected TS-parity status-guard message; got: {err}"
    );
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes, "file must NOT be mutated on failure");
}

#[test]
fn reject_duplicate_event_case_insensitively() {
    // @step given a work unit "RPC-179" with a non-deleted event "UserRegistered" at id 0
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_units(&[("RPC-179", "specifying")]);
    pre["workUnits"]["RPC-179"]["eventStorm"] = json!({
        "level": "process_modeling",
        "items": [{
            "id": 0, "type": "event", "color": "orange",
            "text": "UserRegistered", "deleted": false,
            "createdAt": "2026-06-01T00:00:00.000Z"
        }],
        "nextItemId": 1
    });
    write_value(tmp.path(), &pre);

    // @step when I add a domain event "userregistered"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "RPC-179", "text": "userregistered"}),
    ));

    // @step then the command fails with error "Event 'userregistered' already exists (ID: 0)"
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Event 'userregistered' already exists (ID: 0)"),
        "expected case-insensitive dedup error; got: {err}"
    );
}

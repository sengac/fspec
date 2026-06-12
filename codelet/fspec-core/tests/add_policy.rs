#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-policy-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-policy`
// (RPC-187). Each scenario maps to exactly one #[test] fn with @step
// comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-policy".to_string(),
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

/// Seed a work-units.json with a single (id, status) work unit and no
/// eventStorm field.
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
                "title": format!("title {id}"),
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
fn first_add_seeds_event_storm_sub_object_on_clean_work_unit() {
    // Scenario: First add seeds the eventStorm sub-object on a clean work unit

    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I dispatch add-policy with workUnitId='AUTH-001' and text='Send welcome email'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "text": "Send welcome email"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains policyId=0
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["policyId"].as_u64(), Some(0));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.level='process_modeling'
    let v = read_work_units(tmp.path());
    let es = &v["workUnits"]["AUTH-001"]["eventStorm"];
    assert_eq!(es["level"].as_str(), Some("process_modeling"));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=1
    assert_eq!(es["nextItemId"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0] has type='policy', color='purple', text='Send welcome email', id=0, deleted=false
    let item = &es["items"][0];
    assert_eq!(item["type"].as_str(), Some("policy"));
    assert_eq!(item["color"].as_str(), Some("purple"));
    assert_eq!(item["text"].as_str(), Some("Send welcome email"));
    assert_eq!(item["id"].as_u64(), Some(0));
    assert_eq!(item["deleted"].as_bool(), Some(false));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].createdAt is a fresh ISO-8601 timestamp
    let created = item["createdAt"].as_str().expect("createdAt string");
    assert!(created.len() == 24 && created.ends_with('Z'), "got: {created}");
    assert!(
        !created.starts_with("2026-06-01"),
        "createdAt must NOT be the seed value"
    );
}

#[test]
fn optional_when_then_bounded_context_persisted_in_ts_insertion_order() {
    // Scenario: Optional when/then/boundedContext fields are persisted in TS insertion order

    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I dispatch add-policy with workUnitId='AUTH-001', text='Send welcome email', when='UserRegistered', then='SendWelcomeEmail', boundedContext='Identity'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "text": "Send welcome email",
            "when": "UserRegistered",
            "then": "SendWelcomeEmail",
            "boundedContext": "Identity"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains policyId=0
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["policyId"].as_u64(), Some(0));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].when='UserRegistered'
    let v = read_work_units(tmp.path());
    let item = &v["workUnits"]["AUTH-001"]["eventStorm"]["items"][0];
    assert_eq!(item["when"].as_str(), Some("UserRegistered"));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].then='SendWelcomeEmail'
    assert_eq!(item["then"].as_str(), Some("SendWelcomeEmail"));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].boundedContext='Identity'
    assert_eq!(item["boundedContext"].as_str(), Some("Identity"));

    // @step And the items[0] JSON key order is type, color, text, when, then, boundedContext, id, deleted, createdAt
    let item_keys: Vec<&str> = item
        .as_object()
        .expect("items[0] object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        item_keys,
        vec![
            "type",
            "color",
            "text",
            "when",
            "then",
            "boundedContext",
            "id",
            "deleted",
            "createdAt"
        ],
        "items[0] key order must match TS object-literal insertion order"
    );
}

#[test]
fn optional_timestamp_field_is_persisted_when_provided() {
    // Scenario: Optional timestamp field is persisted when provided

    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I dispatch add-policy with workUnitId='AUTH-001', text='Send welcome email', timestamp=1000
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "text": "Send welcome email",
            "timestamp": 1000
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].timestamp=1000
    let v = read_work_units(tmp.path());
    let item = &v["workUnits"]["AUTH-001"]["eventStorm"]["items"][0];
    assert_eq!(item["timestamp"].as_u64(), Some(1000));
}

#[test]
fn second_add_increments_next_item_id_and_preserves_insertion_order() {
    // Scenario: Second add increments nextItemId and preserves insertion order

    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with an existing eventStorm policy id=0 and nextItemId=1
    let tmp = TempDir::new().expect("tempdir");
    let mut pre: Value = serde_json::from_str(&seed_unit("AUTH-001", "specifying")).unwrap();
    pre["workUnits"]["AUTH-001"]["eventStorm"] = json!({
        "level": "process_modeling",
        "items": [{
            "type": "policy",
            "color": "purple",
            "text": "Send welcome email",
            "id": 0,
            "deleted": false,
            "createdAt": "2026-06-01T00:00:00.000Z"
        }],
        "nextItemId": 1
    });
    write_work_units(tmp.path(), &serde_json::to_string_pretty(&pre).unwrap());

    // @step When I dispatch add-policy with workUnitId='AUTH-001' and text='Notify warehouse'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "text": "Notify warehouse"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains policyId=1
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["policyId"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.nextItemId=2
    let v = read_work_units(tmp.path());
    let es = &v["workUnits"]["AUTH-001"]["eventStorm"];
    assert_eq!(es["nextItemId"].as_u64(), Some(2));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[1] has id=1 and text='Notify warehouse'
    let item = &es["items"][1];
    assert_eq!(item["id"].as_u64(), Some(1));
    assert_eq!(item["text"].as_str(), Some("Notify warehouse"));
}

#[test]
fn missing_work_unit_surfaces_canonical_not_found_error() {
    // Scenario: Missing work unit surfaces the canonical not-found error

    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "specifying"));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-policy with workUnitId='NOPE-001' and text='Anything'
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
        "expected canonical not-found message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(
        pre_bytes, post_bytes,
        "work-units.json must NOT be mutated on failure"
    );
}

#[test]
fn done_state_is_rejected_verbatim() {
    // Scenario: Done state is rejected verbatim

    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=done
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "done"));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-policy with workUnitId='AUTH-001' and text='Anything'
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
        "expected canonical done-state message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn blocked_state_is_rejected_verbatim() {
    // Scenario: Blocked state is rejected verbatim

    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=blocked
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &seed_unit("AUTH-001", "blocked"));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch add-policy with workUnitId='AUTH-001' and text='Anything'
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
        "expected canonical blocked-state message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn reports_canonical_not_found_without_creating_file() {
    // Scenario: Missing spec/work-units.json reports the canonical not-found error without creating the file

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-policy with workUnitId='AUTH-001' and text='Anything'
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
        "expected canonical not-found message; got: {err}"
    );

    // @step And spec/work-units.json does not exist on disk
    assert!(!tmp.path().join("spec/work-units.json").exists());
}

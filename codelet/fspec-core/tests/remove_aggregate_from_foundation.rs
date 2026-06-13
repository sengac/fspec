#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/remove-aggregate-from-foundation-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `remove-aggregate-from-foundation` (RPC-266). Each scenario maps to
// exactly one #[test] function with @step comments mirroring the Gherkin
// steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-aggregate-from-foundation".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_foundation(project_root: &Path, value: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(value).expect("ser foundation"),
    )
    .expect("write foundation.json");
}

fn read_foundation(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/foundation.json"))
        .expect("read foundation.json");
    serde_json::from_str(&raw).expect("parse foundation.json")
}

fn bounded_context(text: &str, id: u64, deleted: bool) -> Value {
    json!({
        "id": id,
        "type": "bounded_context",
        "text": text,
        "color": null,
        "deleted": deleted,
        "createdAt": "2026-01-01T00:00:00.000Z"
    })
}

fn aggregate(text: &str, id: u64, bc_id: u64, deleted: bool) -> Value {
    json!({
        "id": id,
        "type": "aggregate",
        "text": text,
        "boundedContextId": bc_id,
        "color": "yellow",
        "deleted": deleted,
        "createdAt": "2026-01-01T00:00:00.000Z"
    })
}

fn foundation_with_items(items: Vec<Value>) -> Value {
    let next = items
        .iter()
        .filter_map(|i| i["id"].as_u64())
        .max()
        .map_or(0, |m| m + 1);
    json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "eventStorm": {
            "level": "big_picture",
            "items": items,
            "nextItemId": next
        }
    })
}

/// Find an aggregate by text (first match). Panics if absent.
fn find_aggregate(data: &Value, text: &str) -> Value {
    data["eventStorm"]["items"]
        .as_array()
        .expect("items array")
        .iter()
        .find(|i| i["type"].as_str() == Some("aggregate") && i["text"].as_str() == Some(text))
        .cloned()
        .unwrap_or_else(|| panic!("aggregate '{text}' must exist; data={data}"))
}

/// Find an aggregate scoped by (text, boundedContextId). Panics if absent.
fn find_aggregate_in(data: &Value, text: &str, bc_id: u64) -> Value {
    data["eventStorm"]["items"]
        .as_array()
        .expect("items array")
        .iter()
        .find(|i| {
            i["type"].as_str() == Some("aggregate")
                && i["text"].as_str() == Some(text)
                && i["boundedContextId"].as_u64() == Some(bc_id)
        })
        .cloned()
        .unwrap_or_else(|| panic!("aggregate '{text}' in bc {bc_id} must exist; data={data}"))
}

// ---------- scenarios ----------

#[test]
fn dispatcher_soft_deletes_an_existing_aggregate() {
    // Scenario: Dispatcher soft-deletes an existing aggregate

    // @step Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and an aggregate 'Order' linked to it
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(vec![
            bounded_context("Sales", 0, false),
            aggregate("Order", 1, 0, false),
        ]),
    );

    // @step When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Order'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Sales", "aggregateName": "Order"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the aggregate 'Order' in eventStorm.items has deleted=true
    let data = read_foundation(tmp.path());
    assert_eq!(
        find_aggregate(&data, "Order")["deleted"].as_bool(),
        Some(true)
    );

    // @step And the aggregate 'Order' item still exists in eventStorm.items
    let count = data["eventStorm"]["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter(|i| i["type"].as_str() == Some("aggregate") && i["text"].as_str() == Some("Order"))
        .count();
    assert_eq!(count, 1, "Order item must remain in the array");
}

#[test]
fn dispatcher_rejects_when_no_event_storm_section() {
    // Scenario: Dispatcher rejects when foundation.json has no eventStorm section

    // @step Given spec/foundation.json exists with no eventStorm section
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &json!({
            "version": "2.0.0",
            "project": {"name": "x", "vision": "v", "projectType": "cli-tool"}
        }),
    );

    // @step When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Order'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Sales", "aggregateName": "Order"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Bounded context 'Sales' not found (no Event Storm data)"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Bounded context 'Sales' not found (no Event Storm data)"),
        "missing canonical no-data error text; got: {msg}"
    );
}

#[test]
fn dispatcher_rejects_a_non_existent_bounded_context() {
    // Scenario: Dispatcher rejects a non-existent bounded context

    // @step Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and an aggregate 'Order' linked to it
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(vec![
            bounded_context("Sales", 0, false),
            aggregate("Order", 1, 0, false),
        ]),
    );

    // @step When I dispatch remove-aggregate-from-foundation with contextName='Unknown' aggregateName='Order'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Unknown", "aggregateName": "Order"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Bounded context 'Unknown' not found"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Bounded context 'Unknown' not found"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_rejects_a_non_existent_aggregate_within_existing_context() {
    // Scenario: Dispatcher rejects a non-existent aggregate within an existing context

    // @step Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and an aggregate 'Order' linked to it
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(vec![
            bounded_context("Sales", 0, false),
            aggregate("Order", 1, 0, false),
        ]),
    );

    // @step When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Ghost'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Sales", "aggregateName": "Ghost"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Aggregate 'Ghost' not found in bounded context 'Sales'"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Aggregate 'Ghost' not found in bounded context 'Sales'"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_treats_already_deleted_aggregate_as_not_found() {
    // Scenario: Dispatcher treats an already soft-deleted aggregate as not found

    // @step Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and an aggregate 'Order' that is already deleted=true
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(vec![
            bounded_context("Sales", 0, false),
            aggregate("Order", 1, 0, true),
        ]),
    );

    // @step When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Order'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Sales", "aggregateName": "Order"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Aggregate 'Order' not found in bounded context 'Sales'"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Aggregate 'Order' not found in bounded context 'Sales'"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_treats_already_deleted_bounded_context_as_not_found() {
    // Scenario: Dispatcher treats an already soft-deleted bounded context as not found

    // @step Given spec/foundation.json contains a bounded_context 'Sales' with id=0 that is already deleted=true
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(vec![
            bounded_context("Sales", 0, true),
            aggregate("Order", 1, 0, false),
        ]),
    );

    // @step When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Order'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Sales", "aggregateName": "Order"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Bounded context 'Sales' not found"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Bounded context 'Sales' not found"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_only_removes_aggregate_scoped_to_named_context() {
    // Scenario: Dispatcher only removes the aggregate scoped to the named context

    // @step Given spec/foundation.json contains a bounded_context 'Sales' with id=0 and 'Billing' with id=1, each with an aggregate 'Order'
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(vec![
            bounded_context("Sales", 0, false),
            bounded_context("Billing", 1, false),
            aggregate("Order", 2, 0, false),
            aggregate("Order", 3, 1, false),
        ]),
    );

    // @step When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Order'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Sales", "aggregateName": "Order"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the aggregate 'Order' with boundedContextId=0 has deleted=true
    let data = read_foundation(tmp.path());
    assert_eq!(
        find_aggregate_in(&data, "Order", 0)["deleted"].as_bool(),
        Some(true)
    );

    // @step And the aggregate 'Order' with boundedContextId=1 still has deleted=false
    assert_eq!(
        find_aggregate_in(&data, "Order", 1)["deleted"].as_bool(),
        Some(false)
    );
}

#[test]
fn dispatcher_preserves_unknown_top_level_fields_on_write() {
    // Scenario: Dispatcher preserves unknown top-level fields on write

    // @step Given spec/foundation.json contains a bounded_context 'Sales' with id=0, an aggregate 'Order', and a custom top-level 'experiments' key
    let tmp = TempDir::new().expect("tempdir");
    let mut f = foundation_with_items(vec![
        bounded_context("Sales", 0, false),
        aggregate("Order", 1, 0, false),
    ]);
    f["experiments"] = json!({"alpha": true, "beta": [1, 2, 3]});
    write_foundation(tmp.path(), &f);

    // @step When I dispatch remove-aggregate-from-foundation with contextName='Sales' aggregateName='Order'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Sales", "aggregateName": "Order"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json still contains the 'experiments' key with its original value
    let data = read_foundation(tmp.path());
    assert_eq!(data["experiments"]["alpha"].as_bool(), Some(true));
    assert_eq!(data["experiments"]["beta"][0].as_u64(), Some(1));
}

#[test]
fn dispatcher_fails_fast_when_required_args_are_missing() {
    // Scenario: Dispatcher fails fast when required args are missing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch remove-aggregate-from-foundation with no contextName field in the args
    let result = dispatch_command(req(tmp.path(), json!({"aggregateName": "Order"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Invalid args for fspec command remove-aggregate-from-foundation'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Invalid args for fspec command remove-aggregate-from-foundation"),
        "missing canonical InvalidArgs text; got: {msg}"
    );
}

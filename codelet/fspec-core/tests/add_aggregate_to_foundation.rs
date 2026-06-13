#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-aggregate-to-foundation-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `add-aggregate-to-foundation` (RPC-166). Each scenario maps to exactly
// one #[test] function with @step comments mirroring the Gherkin steps
// verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-aggregate-to-foundation".to_string(),
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

/// Build a foundation with an eventStorm section containing the given
/// bounded_context items `(text, id)`. `nextItemId` is one greater than
/// the highest id provided.
fn foundation_with_contexts(contexts: &[(&str, u64)]) -> Value {
    let items: Vec<Value> = contexts
        .iter()
        .map(|(text, id)| {
            json!({
                "id": id,
                "type": "bounded_context",
                "text": text,
                "color": null,
                "deleted": false,
                "createdAt": "2026-01-01T00:00:00.000Z"
            })
        })
        .collect();
    let next = contexts
        .iter()
        .map(|(_, id)| *id)
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

/// Return all non-deleted-or-otherwise aggregate items from eventStorm.items.
fn aggregates(data: &Value) -> Vec<Value> {
    data["eventStorm"]["items"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|i| i["type"].as_str() == Some("aggregate"))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn find_aggregate(data: &Value, text: &str) -> Value {
    aggregates(data)
        .into_iter()
        .find(|a| a["text"].as_str() == Some(text))
        .unwrap_or_else(|| panic!("aggregate '{text}' must exist; data={data}"))
}

// ---------- scenarios ----------

#[test]
fn dispatcher_appends_a_new_aggregate_to_an_existing_bounded_context() {
    // Scenario: Dispatcher appends a new aggregate to an existing bounded context

    // @step Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &foundation_with_contexts(&[("Sales", 0)]));

    // @step When I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Order'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Sales", "aggregateName": "Order"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/foundation.json eventStorm.items contains exactly one aggregate item
    let data = read_foundation(tmp.path());
    assert_eq!(
        aggregates(&data).len(),
        1,
        "expected 1 aggregate; data={data}"
    );

    // @step And that aggregate has type='aggregate', text='Order', color='yellow', and deleted=false
    let agg = find_aggregate(&data, "Order");
    assert_eq!(agg["type"].as_str(), Some("aggregate"));
    assert_eq!(agg["text"].as_str(), Some("Order"));
    assert_eq!(agg["color"].as_str(), Some("yellow"));
    assert_eq!(agg["deleted"].as_bool(), Some(false));

    // @step And that aggregate has boundedContextId=0 matching the 'Sales' context
    assert_eq!(agg["boundedContextId"].as_u64(), Some(0));

    // @step And eventStorm.nextItemId has been incremented
    assert_eq!(data["eventStorm"]["nextItemId"].as_u64(), Some(2));
}

#[test]
fn dispatcher_assigns_sequential_ids_and_increments_next_item_id_per_aggregate() {
    // Scenario: Dispatcher assigns sequential ids and increments nextItemId per aggregate

    // @step Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &foundation_with_contexts(&[("Sales", 0)]));

    // @step When I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Order'
    let r1 = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Sales", "aggregateName": "Order"}),
    ));
    assert!(r1.success, "{r1:?}");

    // @step And I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Shipment'
    let r2 = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Sales", "aggregateName": "Shipment"}),
    ));
    assert!(r2.success, "{r2:?}");

    // @step Then spec/foundation.json eventStorm.items contains two aggregate items
    let data = read_foundation(tmp.path());
    assert_eq!(aggregates(&data).len(), 2, "data={data}");

    // @step And the second aggregate has an id one greater than the first aggregate
    let first = find_aggregate(&data, "Order");
    let second = find_aggregate(&data, "Shipment");
    let id1 = first["id"].as_u64().expect("first id");
    let id2 = second["id"].as_u64().expect("second id");
    assert_eq!(id2, id1 + 1, "second id must be first+1");

    // @step And eventStorm.nextItemId equals one greater than the second aggregate id
    assert_eq!(data["eventStorm"]["nextItemId"].as_u64(), Some(id2 + 1));
}

#[test]
fn dispatcher_persists_optional_description_when_provided() {
    // Scenario: Dispatcher persists optional description when provided

    // @step Given spec/foundation.json contains a bounded_context item 'Billing' with id=0 in eventStorm.items
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &foundation_with_contexts(&[("Billing", 0)]));

    // @step When I dispatch add-aggregate-to-foundation with contextName='Billing' aggregateName='Invoice' description='Billing root'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "contextName": "Billing",
            "aggregateName": "Invoice",
            "description": "Billing root"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the aggregate 'Invoice' has description='Billing root'
    let data = read_foundation(tmp.path());
    let agg = find_aggregate(&data, "Invoice");
    assert_eq!(agg["description"].as_str(), Some("Billing root"));
}

#[test]
fn dispatcher_omits_description_field_when_not_provided() {
    // Scenario: Dispatcher omits the description field when no description is provided

    // @step Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &foundation_with_contexts(&[("Sales", 0)]));

    // @step When I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Order'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Sales", "aggregateName": "Order"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the aggregate 'Order' has no 'description' field
    let data = read_foundation(tmp.path());
    let agg = find_aggregate(&data, "Order");
    assert!(
        agg.get("description").is_none(),
        "aggregate must NOT include 'description'; got {agg}"
    );
}

#[test]
fn dispatcher_rejects_a_non_existent_bounded_context() {
    // Scenario: Dispatcher rejects a non-existent bounded context

    // @step Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 in eventStorm.items
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &foundation_with_contexts(&[("Sales", 0)]));

    // @step When I dispatch add-aggregate-to-foundation with contextName='Unknown' aggregateName='Order'
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
fn dispatcher_rejects_when_foundation_has_no_event_storm_data() {
    // Scenario: Dispatcher rejects when foundation.json has no eventStorm data

    // @step Given spec/foundation.json exists with no eventStorm section
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &json!({
            "version": "2.0.0",
            "project": {"name": "x", "vision": "v", "projectType": "cli-tool"}
        }),
    );

    // @step When I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Order'
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
fn dispatcher_auto_creates_foundation_but_still_fails_when_no_bounded_context_exists() {
    // Scenario: Dispatcher auto-creates foundation.json but still fails when no bounded context exists

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Order'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Sales", "aggregateName": "Order"}),
    ));

    // @step Then the file spec/foundation.json exists
    assert!(
        tmp.path().join("spec/foundation.json").exists(),
        "spec/foundation.json must be auto-created by read_or_init_json"
    );

    // @step And the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Bounded context 'Sales' not found"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Bounded context 'Sales' not found"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_links_aggregate_to_correct_context_among_multiple() {
    // Scenario: Dispatcher links the aggregate to the correct context among multiple bounded contexts

    // @step Given spec/foundation.json contains bounded_context items 'Sales' with id=0 and 'Billing' with id=1 in eventStorm.items
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_contexts(&[("Sales", 0), ("Billing", 1)]),
    );

    // @step When I dispatch add-aggregate-to-foundation with contextName='Billing' aggregateName='Invoice'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Billing", "aggregateName": "Invoice"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the aggregate 'Invoice' has boundedContextId=1 matching the 'Billing' context
    let data = read_foundation(tmp.path());
    let agg = find_aggregate(&data, "Invoice");
    assert_eq!(agg["boundedContextId"].as_u64(), Some(1));
}

#[test]
fn dispatcher_preserves_unknown_top_level_fields_on_write() {
    // Scenario: Dispatcher preserves unknown top-level fields on write

    // @step Given spec/foundation.json contains a bounded_context item 'Sales' with id=0 and a custom top-level 'experiments' key
    let tmp = TempDir::new().expect("tempdir");
    let mut f = foundation_with_contexts(&[("Sales", 0)]);
    f["experiments"] = json!({"alpha": true, "beta": [1, 2, 3]});
    write_foundation(tmp.path(), &f);

    // @step When I dispatch add-aggregate-to-foundation with contextName='Sales' aggregateName='Order'
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

    // @step When I dispatch add-aggregate-to-foundation with no contextName field in the args
    let result = dispatch_command(req(tmp.path(), json!({"aggregateName": "Order"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring 'Invalid args for fspec command add-aggregate-to-foundation'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Invalid args for fspec command add-aggregate-to-foundation"),
        "missing canonical InvalidArgs text; got: {msg}"
    );
}

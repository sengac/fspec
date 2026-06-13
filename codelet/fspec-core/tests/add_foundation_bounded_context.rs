#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-foundation-bounded-context-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `add-foundation-bounded-context` (RPC-183). Each scenario maps to exactly
// one #[test] function with @step comments mirroring the Gherkin steps
// verbatim. Tests target spec/foundation.json eventStorm.items at the
// big_picture level.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-foundation-bounded-context".to_string(),
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

fn read_foundation_raw(project_root: &Path) -> String {
    fs::read_to_string(project_root.join("spec/foundation.json")).expect("read foundation.json")
}

fn empty_foundation() -> Value {
    json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "solutionSpace": {"overview": "o", "capabilities": []}
    })
}

// ---------- scenarios ----------

#[test]
fn first_add_seeds_event_storm_sub_object() {
    // Scenario: First add seeds the eventStorm sub-object on a foundation with no eventStorm field

    // @step Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-foundation-bounded-context with text='Order Management'
    let result = dispatch_command(req(tmp.path(), json!({"text": "Order Management"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains message='Added bounded context "Order Management" to foundation Event Storm'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["message"].as_str(),
        Some("Added bounded context \"Order Management\" to foundation Event Storm")
    );

    // @step And spec/foundation.json on disk shows eventStorm.level='big_picture'
    let f = read_foundation(tmp.path());
    assert_eq!(f["eventStorm"]["level"].as_str(), Some("big_picture"));

    // @step And spec/foundation.json on disk shows eventStorm.nextItemId=2
    assert_eq!(f["eventStorm"]["nextItemId"].as_u64(), Some(2));

    // @step And spec/foundation.json on disk shows eventStorm.items[0] has id=1, type='bounded_context', text='Order Management', deleted=false
    let i0 = &f["eventStorm"]["items"][0];
    assert_eq!(i0["id"].as_u64(), Some(1));
    assert_eq!(i0["type"].as_str(), Some("bounded_context"));
    assert_eq!(i0["text"].as_str(), Some("Order Management"));
    assert_eq!(i0["deleted"].as_bool(), Some(false));

    // @step And spec/foundation.json on disk shows eventStorm.items[0].createdAt is a fresh ISO-8601 timestamp
    let created = i0["createdAt"]
        .as_str()
        .expect("createdAt must be a string");
    assert_eq!(
        created.len(),
        24,
        "ISO-8601 ms timestamp is 24 chars: {created}"
    );
    assert!(
        created.ends_with('Z'),
        "createdAt must end with Z: {created}"
    );
}

#[test]
fn color_field_is_persisted_as_json_null() {
    // Scenario: The color field is persisted as JSON null

    // @step Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-foundation-bounded-context with text='Identity'
    let result = dispatch_command(req(tmp.path(), json!({"text": "Identity"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json on disk shows eventStorm.items[0].color is JSON null (key present with null value)
    let f = read_foundation(tmp.path());
    let i0 = f["eventStorm"]["items"][0]
        .as_object()
        .expect("items[0] is object");
    assert!(
        i0.contains_key("color"),
        "color key must be present: {i0:?}"
    );
    assert!(
        i0["color"].is_null(),
        "color must be JSON null: {:?}",
        i0["color"]
    );
}

#[test]
fn persisted_item_key_order_matches_ts_insertion_order() {
    // Scenario: The persisted item key order matches TS insertion order

    // @step Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &empty_foundation());

    // @step When I dispatch add-foundation-bounded-context with text='Catalog'
    let result = dispatch_command(req(tmp.path(), json!({"text": "Catalog"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the eventStorm.items[0] JSON key order is exactly id, type, text, color, deleted, createdAt
    let raw = read_foundation_raw(tmp.path());
    let expected_order = [
        "\"id\"",
        "\"type\"",
        "\"text\"",
        "\"color\"",
        "\"deleted\"",
        "\"createdAt\"",
    ];
    let mut last = 0usize;
    for key in expected_order {
        let pos = raw[last..]
            .find(key)
            .map(|p| p + last)
            .unwrap_or_else(|| panic!("key {key} not found after position {last} in:\n{raw}"));
        assert!(pos >= last, "key {key} out of order in:\n{raw}");
        last = pos + key.len();
    }
}

#[test]
fn second_add_increments_next_item_id_and_assigns_next_id() {
    // Scenario: Second add increments nextItemId and assigns the next id

    // @step Given a project root tempdir with spec/foundation.json containing an existing eventStorm bounded_context id=1 and nextItemId=2
    let tmp = TempDir::new().expect("tempdir");
    let mut f = empty_foundation();
    f["eventStorm"] = json!({
        "level": "big_picture",
        "items": [{
            "id": 1,
            "type": "bounded_context",
            "text": "Order Management",
            "color": null,
            "deleted": false,
            "createdAt": "2026-01-01T00:00:00.000Z"
        }],
        "nextItemId": 2
    });
    write_foundation(tmp.path(), &f);

    // @step When I dispatch add-foundation-bounded-context with text='Shipping'
    let result = dispatch_command(req(tmp.path(), json!({"text": "Shipping"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json on disk shows eventStorm.nextItemId=3
    let data = read_foundation(tmp.path());
    assert_eq!(data["eventStorm"]["nextItemId"].as_u64(), Some(3));

    // @step And spec/foundation.json on disk shows eventStorm.items[1] has id=2 and text='Shipping'
    let i1 = &data["eventStorm"]["items"][1];
    assert_eq!(i1["id"].as_u64(), Some(2));
    assert_eq!(i1["text"].as_str(), Some("Shipping"));
}

#[test]
fn missing_foundation_json_is_auto_created_before_appending() {
    // Scenario: Missing foundation.json is auto-created with the default schema before appending

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-foundation-bounded-context with text='Billing'
    let result = dispatch_command(req(tmp.path(), json!({"text": "Billing"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json exists on disk
    assert!(
        tmp.path().join("spec/foundation.json").exists(),
        "spec/foundation.json must be auto-created with the TS slim inline default"
    );

    // @step And spec/foundation.json on disk shows eventStorm.items[0] has id=1 and text='Billing'
    let f = read_foundation(tmp.path());
    let i0 = &f["eventStorm"]["items"][0];
    assert_eq!(i0["id"].as_u64(), Some(1));
    assert_eq!(i0["text"].as_str(), Some("Billing"));
}

#[test]
fn unknown_top_level_fields_are_preserved() {
    // Scenario: Unknown top-level foundation fields are preserved byte-for-byte

    // @step Given a project root tempdir with spec/foundation.json containing a custom top-level field extraField='keep-me' and no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    let mut f = empty_foundation();
    f["extraField"] = json!("keep-me");
    write_foundation(tmp.path(), &f);

    // @step When I dispatch add-foundation-bounded-context with text='Payments'
    let result = dispatch_command(req(tmp.path(), json!({"text": "Payments"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/foundation.json on disk still contains extraField='keep-me'
    let data = read_foundation(tmp.path());
    assert_eq!(data["extraField"].as_str(), Some("keep-me"));

    // @step And spec/foundation.json on disk shows eventStorm.items[0].text='Payments'
    assert_eq!(
        data["eventStorm"]["items"][0]["text"].as_str(),
        Some("Payments")
    );
}

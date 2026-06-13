#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-domain-event-to-foundation-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `add-domain-event-to-foundation` (RPC-180). Each scenario maps to exactly one
// #[test] function with @step comments mirroring the Gherkin steps verbatim.
//
// Twin: codelet/fspec-core/tests/add_command_to_foundation.rs (RPC-175). Diffs
// here: item type='event', color='orange', 2nd positional eventName, message
// noun 'domain event'.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-domain-event-to-foundation".to_string(),
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

/// Build a foundation.json Value whose eventStorm holds the supplied
/// bounded_context items (id, text) and the given nextItemId.
fn foundation_with_contexts(contexts: &[(u64, &str)], next_item_id: u64) -> Value {
    let items: Vec<Value> = contexts
        .iter()
        .map(|(id, text)| {
            json!({
                "id": id,
                "type": "bounded_context",
                "text": text,
                "color": null,
                "deleted": false,
                "createdAt": "2026-06-01T00:00:00.000Z"
            })
        })
        .collect();
    json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "eventStorm": {
            "level": "big_picture",
            "items": items,
            "nextItemId": next_item_id
        }
    })
}

/// Return the first event item in eventStorm.items.
fn first_event(foundation: &Value) -> Option<&Value> {
    foundation["eventStorm"]["items"]
        .as_array()?
        .iter()
        .find(|i| i["type"].as_str() == Some("event"))
}

// ---------- scenarios ----------

#[test]
fn adding_a_domain_event_appends_the_item_and_increments_next_item_id() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_contexts(&[(0, "Work Management")], 1),
    );

    // @step When I dispatch add-domain-event-to-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "eventName": "WorkUnitCreated"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned message is 'Added domain event "WorkUnitCreated" to "Work Management" bounded context'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["message"].as_str(),
        Some("Added domain event \"WorkUnitCreated\" to \"Work Management\" bounded context")
    );

    // @step And spec/foundation.json on disk shows eventStorm.nextItemId=2
    let v = read_foundation(tmp.path());
    assert_eq!(v["eventStorm"]["nextItemId"].as_u64(), Some(2));

    // @step And spec/foundation.json on disk shows the appended item has type='event', text='WorkUnitCreated', boundedContextId=0, id=1, deleted=false
    let ev = first_event(&v).expect("an event item must exist");
    assert_eq!(ev["type"].as_str(), Some("event"));
    assert_eq!(ev["text"].as_str(), Some("WorkUnitCreated"));
    assert_eq!(ev["boundedContextId"].as_u64(), Some(0));
    assert_eq!(ev["id"].as_u64(), Some(1));
    assert_eq!(ev["deleted"].as_bool(), Some(false));

    // @step And the appended item createdAt is a fresh ISO-8601 timestamp
    let created = ev["createdAt"].as_str().expect("createdAt string");
    assert!(
        created.len() == 24 && created.ends_with('Z'),
        "got: {created}"
    );
}

#[test]
fn color_field_is_persisted_as_the_json_string_orange() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_contexts(&[(0, "Work Management")], 1),
    );

    // @step When I dispatch add-domain-event-to-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "eventName": "WorkUnitCreated"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/foundation.json on disk shows the appended item color='orange' (a JSON string, not blue, not null)
    let v = read_foundation(tmp.path());
    let ev = first_event(&v).expect("an event item must exist");
    assert_eq!(
        ev["color"].as_str(),
        Some("orange"),
        "color must be the JSON string 'orange'"
    );
}

#[test]
fn optional_description_is_persisted_as_the_trailing_field_in_ts_key_order() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_contexts(&[(0, "Work Management")], 1),
    );

    // @step When I dispatch add-domain-event-to-foundation with contextName='Work Management', eventName='WorkUnitCreated', description='Signals work unit reached done status'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "contextName": "Work Management",
            "eventName": "WorkUnitCreated",
            "description": "Signals work unit reached done status"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/foundation.json on disk shows the appended item description='Signals work unit reached done status'
    let v = read_foundation(tmp.path());
    let ev = first_event(&v).expect("an event item must exist");
    assert_eq!(
        ev["description"].as_str(),
        Some("Signals work unit reached done status")
    );

    // @step And the appended item JSON key order is id, type, text, boundedContextId, color, deleted, createdAt, description
    let raw = read_foundation_raw(tmp.path());
    let expected_order = [
        "\"id\"",
        "\"type\"",
        "\"text\"",
        "\"boundedContextId\"",
        "\"color\"",
        "\"deleted\"",
        "\"createdAt\"",
        "\"description\"",
    ];
    // The event item is the one carrying "boundedContextId", so anchor the
    // search from that key's position and walk back to the object open brace.
    let anchor = raw
        .find("\"boundedContextId\"")
        .expect("event item with boundedContextId must be serialized");
    let obj_start = raw[..anchor].rfind('{').expect("event object open brace");
    let mut last = obj_start;
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
fn adding_to_a_non_existent_bounded_context_fails_and_leaves_file_unchanged() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_contexts(&[(0, "Work Management")], 1),
    );
    let pre_bytes = fs::read(tmp.path().join("spec/foundation.json")).unwrap();

    // @step When I dispatch add-domain-event-to-foundation with contextName='Nope' and eventName='WorkUnitCreated'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Nope", "eventName": "WorkUnitCreated"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Bounded context 'Nope' not found"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Bounded context 'Nope' not found"),
        "expected canonical missing-context message; got: {err}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/foundation.json")).unwrap();
    assert_eq!(
        pre_bytes, post_bytes,
        "foundation.json must NOT be mutated on failure"
    );
}

#[test]
fn event_links_only_to_matching_context_and_second_add_increments_next_item_id() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has bounded_context text='Work Management' id=0 and bounded_context text='Specification' id=1 and nextItemId=2
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_contexts(&[(0, "Work Management"), (1, "Specification")], 2),
    );

    // @step When I dispatch add-domain-event-to-foundation with contextName='Specification' and eventName='FeatureCreated'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Specification", "eventName": "FeatureCreated"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the appended item has boundedContextId=1 and id=2
    let v = read_foundation(tmp.path());
    let feature_created = v["eventStorm"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["text"].as_str() == Some("FeatureCreated"))
        .expect("FeatureCreated event must exist");
    assert_eq!(feature_created["boundedContextId"].as_u64(), Some(1));
    assert_eq!(feature_created["id"].as_u64(), Some(2));

    // @step When I dispatch add-domain-event-to-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    let result2 = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "eventName": "WorkUnitCreated"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result2.success, "expected success=true, got {result2:?}");

    // @step And the second appended item has boundedContextId=0 and id=3
    let v2 = read_foundation(tmp.path());
    let wu_created = v2["eventStorm"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["text"].as_str() == Some("WorkUnitCreated"))
        .expect("WorkUnitCreated event must exist");
    assert_eq!(wu_created["boundedContextId"].as_u64(), Some(0));
    assert_eq!(wu_created["id"].as_u64(), Some(3));

    // @step And spec/foundation.json on disk shows eventStorm.nextItemId=4
    assert_eq!(v2["eventStorm"]["nextItemId"].as_u64(), Some(4));

    // @step And both event items are present in eventStorm.items
    let event_count = v2["eventStorm"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["type"].as_str() == Some("event"))
        .count();
    assert_eq!(event_count, 2, "both event items must be present");
}

#[test]
fn foundation_with_no_event_storm_reports_canonical_not_found_error() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-domain-event-to-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "eventName": "WorkUnitCreated"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Bounded context 'Work Management' not found"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Bounded context 'Work Management' not found"),
        "expected canonical missing-context message; got: {err}"
    );
}

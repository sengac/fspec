#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/remove-domain-event-from-foundation-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `remove-domain-event-from-foundation` (RPC-272). Each scenario maps to
// exactly one #[test] function with @step comments mirroring the Gherkin steps
// verbatim.
//
// Twin: codelet/fspec-core/tests/remove_command_from_foundation.rs (RPC-270).
// Diffs here: item type matched 'event', not-found noun 'Domain event',
// message noun 'domain event', 2nd positional eventName.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-domain-event-from-foundation".to_string(),
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

fn bounded_context(id: u64, text: &str) -> Value {
    json!({
        "id": id,
        "type": "bounded_context",
        "text": text,
        "color": null,
        "deleted": false,
        "createdAt": "2026-06-01T00:00:00.000Z"
    })
}

fn event_item(id: u64, text: &str, bc_id: u64, deleted: bool) -> Value {
    json!({
        "id": id,
        "type": "event",
        "text": text,
        "boundedContextId": bc_id,
        "color": "orange",
        "deleted": deleted,
        "createdAt": "2026-06-01T00:00:00.000Z"
    })
}

fn foundation_with_items(items: Vec<Value>, next_item_id: u64) -> Value {
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

/// Find an event item by text within the eventStorm.items array.
fn find_event_by_text<'a>(foundation: &'a Value, text: &str) -> Option<&'a Value> {
    foundation["eventStorm"]["items"]
        .as_array()?
        .iter()
        .find(|i| i["type"].as_str() == Some("event") && i["text"].as_str() == Some(text))
}

// ---------- scenarios ----------

#[test]
fn removing_an_existing_event_soft_deletes_it_and_returns_success_message() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and an event text='WorkUnitCreated' boundedContextId=0 deleted=false
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(
            vec![
                bounded_context(0, "Work Management"),
                event_item(1, "WorkUnitCreated", 0, false),
            ],
            2,
        ),
    );

    // @step When I dispatch remove-domain-event-from-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "eventName": "WorkUnitCreated"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned message is 'Removed domain event "WorkUnitCreated" from "Work Management" bounded context'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["message"].as_str(),
        Some("Removed domain event \"WorkUnitCreated\" from \"Work Management\" bounded context")
    );

    // @step And spec/foundation.json on disk shows the WorkUnitCreated event item deleted=true
    let v = read_foundation(tmp.path());
    let ev = find_event_by_text(&v, "WorkUnitCreated").expect("event must still exist");
    assert_eq!(
        ev["deleted"].as_bool(),
        Some(true),
        "event must be soft-deleted"
    );

    // @step And the bounded_context item and all other items are unchanged
    let bc = v["eventStorm"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["type"].as_str() == Some("bounded_context"))
        .expect("bounded_context must exist");
    assert_eq!(bc["deleted"].as_bool(), Some(false));
    assert_eq!(bc["text"].as_str(), Some("Work Management"));
}

#[test]
fn removing_when_foundation_has_no_event_storm_reports_no_data_error_unchanged() {
    // @step Given a project root tempdir with spec/foundation.json that has no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &json!({
            "version": "2.0.0",
            "project": {"name": "x", "vision": "v", "projectType": "cli-tool"}
        }),
    );
    let pre_bytes = fs::read(tmp.path().join("spec/foundation.json")).unwrap();

    // @step When I dispatch remove-domain-event-from-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "eventName": "WorkUnitCreated"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Bounded context 'Work Management' not found (no Event Storm data)"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Bounded context 'Work Management' not found (no Event Storm data)"),
        "expected no-event-storm message; got: {err}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/foundation.json")).unwrap();
    assert_eq!(
        pre_bytes, post_bytes,
        "foundation.json must NOT be mutated on failure"
    );
}

#[test]
fn removing_from_a_non_existent_bounded_context_fails_with_context_not_found() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and an event text='WorkUnitCreated' boundedContextId=0
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(
            vec![
                bounded_context(0, "Work Management"),
                event_item(1, "WorkUnitCreated", 0, false),
            ],
            2,
        ),
    );
    let pre_bytes = fs::read(tmp.path().join("spec/foundation.json")).unwrap();

    // @step When I dispatch remove-domain-event-from-foundation with contextName='Nope' and eventName='WorkUnitCreated'
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
        "expected missing-context message; got: {err}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/foundation.json")).unwrap();
    assert_eq!(
        pre_bytes, post_bytes,
        "foundation.json must NOT be mutated on failure"
    );
}

#[test]
fn removing_an_event_name_not_in_matched_context_fails() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and an event text='WorkUnitCreated' boundedContextId=0
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(
            vec![
                bounded_context(0, "Work Management"),
                event_item(1, "WorkUnitCreated", 0, false),
            ],
            2,
        ),
    );
    let pre_bytes = fs::read(tmp.path().join("spec/foundation.json")).unwrap();

    // @step When I dispatch remove-domain-event-from-foundation with contextName='Work Management' and eventName='Ghost'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "eventName": "Ghost"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Domain event 'Ghost' not found in bounded context 'Work Management'"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Domain event 'Ghost' not found in bounded context 'Work Management'"),
        "expected missing-event message; got: {err}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/foundation.json")).unwrap();
    assert_eq!(
        pre_bytes, post_bytes,
        "foundation.json must NOT be mutated on failure"
    );
}

#[test]
fn event_belonging_to_a_different_context_id_is_not_matched() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has bounded_context text='Work Management' id=0, bounded_context text='Specification' id=1, and an event text='FeatureCreated' boundedContextId=1
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(
            vec![
                bounded_context(0, "Work Management"),
                bounded_context(1, "Specification"),
                event_item(2, "FeatureCreated", 1, false),
            ],
            3,
        ),
    );

    // @step When I dispatch remove-domain-event-from-foundation with contextName='Work Management' and eventName='FeatureCreated'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "eventName": "FeatureCreated"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Domain event 'FeatureCreated' not found in bounded context 'Work Management'"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains(
            "Domain event 'FeatureCreated' not found in bounded context 'Work Management'"
        ),
        "expected context-mismatch message; got: {err}"
    );
}

#[test]
fn removing_an_already_soft_deleted_event_fails_as_not_found() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and an event text='WorkUnitCreated' boundedContextId=0 deleted=true
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(
            vec![
                bounded_context(0, "Work Management"),
                event_item(1, "WorkUnitCreated", 0, true),
            ],
            2,
        ),
    );

    // @step When I dispatch remove-domain-event-from-foundation with contextName='Work Management' and eventName='WorkUnitCreated'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "eventName": "WorkUnitCreated"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Domain event 'WorkUnitCreated' not found in bounded context 'Work Management'"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains(
            "Domain event 'WorkUnitCreated' not found in bounded context 'Work Management'"
        ),
        "expected not-found-for-deleted message; got: {err}"
    );
}

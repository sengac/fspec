#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/remove-foundation-bounded-context-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `remove-foundation-bounded-context` (RPC-274). Each scenario maps to
// exactly one #[test] function with @step comments mirroring the Gherkin
// steps verbatim. Removal is a SOFT-delete (deleted=true), optionally
// cascading to child items carrying a boundedContextId.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-foundation-bounded-context".to_string(),
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

fn base_foundation() -> Value {
    json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "solutionSpace": {"overview": "o", "capabilities": []}
    })
}

/// Find the eventStorm item with the given `text` (regardless of deleted state).
fn item_by_text<'a>(f: &'a Value, text: &str) -> &'a Value {
    f["eventStorm"]["items"]
        .as_array()
        .expect("items array")
        .iter()
        .find(|i| i["text"].as_str() == Some(text))
        .unwrap_or_else(|| panic!("no item with text={text}"))
}

// ---------- scenarios ----------

#[test]
fn remove_childless_bounded_context_soft_deletes_it() {
    // Scenario: Remove a childless bounded context soft-deletes it

    // @step Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Identity' deleted=false and no children
    let tmp = TempDir::new().expect("tempdir");
    let mut f = base_foundation();
    f["eventStorm"] = json!({
        "level": "big_picture",
        "items": [{
            "id": 1, "type": "bounded_context", "text": "Identity",
            "color": null, "deleted": false, "createdAt": "2026-01-01T00:00:00.000Z"
        }],
        "nextItemId": 2
    });
    write_foundation(tmp.path(), &f);

    // @step When I dispatch remove-foundation-bounded-context with contextName='Identity'
    let result = dispatch_command(req(tmp.path(), json!({"contextName": "Identity"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains message='Removed bounded context "Identity" from foundation Event Storm'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["message"].as_str(),
        Some("Removed bounded context \"Identity\" from foundation Event Storm")
    );

    // @step And spec/foundation.json on disk shows the 'Identity' bounded_context item has deleted=true
    let disk = read_foundation(tmp.path());
    assert_eq!(
        item_by_text(&disk, "Identity")["deleted"].as_bool(),
        Some(true)
    );

    // @step And the 'Identity' bounded_context item still exists in eventStorm.items (soft-delete, not spliced)
    assert_eq!(
        disk["eventStorm"]["items"].as_array().expect("array").len(),
        1
    );
}

#[test]
fn refuse_to_remove_non_empty_context_without_cascade() {
    // Scenario: Refuse to remove a non-empty bounded context without --cascade

    // @step Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Sales' with 2 non-deleted child items carrying its boundedContextId
    let tmp = TempDir::new().expect("tempdir");
    let mut f = base_foundation();
    f["eventStorm"] = json!({
        "level": "big_picture",
        "items": [
            {"id": 1, "type": "bounded_context", "text": "Sales", "color": null, "deleted": false, "createdAt": "2026-01-01T00:00:00.000Z"},
            {"id": 2, "type": "aggregate", "text": "Order", "color": "yellow", "boundedContextId": 1, "deleted": false, "createdAt": "2026-01-01T00:00:01.000Z"},
            {"id": 3, "type": "event", "text": "OrderPlaced", "color": "orange", "boundedContextId": 1, "deleted": false, "createdAt": "2026-01-01T00:00:02.000Z"}
        ],
        "nextItemId": 4
    });
    write_foundation(tmp.path(), &f);
    let pre = read_foundation_raw(tmp.path());

    // @step When I dispatch remove-foundation-bounded-context with contextName='Sales'
    let result = dispatch_command(req(tmp.path(), json!({"contextName": "Sales"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Bounded context 'Sales' has 2 child items. Use --cascade to remove the context and all its children."
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Bounded context 'Sales' has 2 child items. Use --cascade to remove the context and all its children."),
        "missing canonical refusal text; got: {msg}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    assert_eq!(read_foundation_raw(tmp.path()), pre);
}

#[test]
fn remove_non_empty_context_with_cascade_soft_deletes_context_and_children() {
    // Scenario: Remove a non-empty bounded context with --cascade soft-deletes context and children

    // @step Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Sales' with 2 non-deleted child items carrying its boundedContextId
    let tmp = TempDir::new().expect("tempdir");
    let mut f = base_foundation();
    f["eventStorm"] = json!({
        "level": "big_picture",
        "items": [
            {"id": 1, "type": "bounded_context", "text": "Sales", "color": null, "deleted": false, "createdAt": "2026-01-01T00:00:00.000Z"},
            {"id": 2, "type": "aggregate", "text": "Order", "color": "yellow", "boundedContextId": 1, "deleted": false, "createdAt": "2026-01-01T00:00:01.000Z"},
            {"id": 3, "type": "event", "text": "OrderPlaced", "color": "orange", "boundedContextId": 1, "deleted": false, "createdAt": "2026-01-01T00:00:02.000Z"}
        ],
        "nextItemId": 4
    });
    write_foundation(tmp.path(), &f);

    // @step When I dispatch remove-foundation-bounded-context with contextName='Sales' and cascade=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Sales", "cascade": true}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains message='Removed bounded context "Sales" and all its children from foundation Event Storm'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["message"].as_str(),
        Some("Removed bounded context \"Sales\" and all its children from foundation Event Storm")
    );

    // @step And spec/foundation.json on disk shows the 'Sales' bounded_context item has deleted=true
    let disk = read_foundation(tmp.path());
    assert_eq!(
        item_by_text(&disk, "Sales")["deleted"].as_bool(),
        Some(true)
    );

    // @step And spec/foundation.json on disk shows both child items have deleted=true
    assert_eq!(
        item_by_text(&disk, "Order")["deleted"].as_bool(),
        Some(true)
    );
    assert_eq!(
        item_by_text(&disk, "OrderPlaced")["deleted"].as_bool(),
        Some(true)
    );
}

#[test]
fn removing_unmatched_name_errors_not_found() {
    // Scenario: Removing a name with no matching non-deleted bounded context errors

    // @step Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Identity' deleted=false
    let tmp = TempDir::new().expect("tempdir");
    let mut f = base_foundation();
    f["eventStorm"] = json!({
        "level": "big_picture",
        "items": [{"id": 1, "type": "bounded_context", "text": "Identity", "color": null, "deleted": false, "createdAt": "2026-01-01T00:00:00.000Z"}],
        "nextItemId": 2
    });
    write_foundation(tmp.path(), &f);
    let pre = read_foundation_raw(tmp.path());

    // @step When I dispatch remove-foundation-bounded-context with contextName='Nope'
    let result = dispatch_command(req(tmp.path(), json!({"contextName": "Nope"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Bounded context 'Nope' not found"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Bounded context 'Nope' not found"),
        "missing canonical not-found text; got: {msg}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    assert_eq!(read_foundation_raw(tmp.path()), pre);
}

#[test]
fn already_soft_deleted_context_is_treated_as_not_found() {
    // Scenario: An already soft-deleted bounded context is treated as not found

    // @step Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Legacy' deleted=true
    let tmp = TempDir::new().expect("tempdir");
    let mut f = base_foundation();
    f["eventStorm"] = json!({
        "level": "big_picture",
        "items": [{"id": 1, "type": "bounded_context", "text": "Legacy", "color": null, "deleted": true, "createdAt": "2026-01-01T00:00:00.000Z"}],
        "nextItemId": 2
    });
    write_foundation(tmp.path(), &f);
    let pre = read_foundation_raw(tmp.path());

    // @step When I dispatch remove-foundation-bounded-context with contextName='Legacy'
    let result = dispatch_command(req(tmp.path(), json!({"contextName": "Legacy"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Bounded context 'Legacy' not found"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Bounded context 'Legacy' not found"),
        "missing canonical not-found text; got: {msg}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    assert_eq!(read_foundation_raw(tmp.path()), pre);
}

#[test]
fn removing_against_foundation_without_event_storm_errors() {
    // Scenario: Removing against a foundation with no eventStorm field errors

    // @step Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &base_foundation());

    // @step When I dispatch remove-foundation-bounded-context with contextName='Anything'
    let result = dispatch_command(req(tmp.path(), json!({"contextName": "Anything"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Bounded context 'Anything' not found (no Event Storm data)"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Bounded context 'Anything' not found (no Event Storm data)"),
        "missing canonical no-event-storm text; got: {msg}"
    );
}

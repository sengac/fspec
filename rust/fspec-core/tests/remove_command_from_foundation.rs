#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/remove-command-from-foundation-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `remove-command-from-foundation` (RPC-270). Each scenario maps to exactly
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
        command: "remove-command-from-foundation".to_string(),
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

fn command_item(id: u64, text: &str, bc_id: u64, deleted: bool) -> Value {
    json!({
        "id": id,
        "type": "command",
        "text": text,
        "boundedContextId": bc_id,
        "color": "blue",
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

/// Find a command item by text within the eventStorm.items array.
fn find_command_by_text<'a>(foundation: &'a Value, text: &str) -> Option<&'a Value> {
    foundation["eventStorm"]["items"]
        .as_array()?
        .iter()
        .find(|i| i["type"].as_str() == Some("command") && i["text"].as_str() == Some(text))
}

// ---------- scenarios ----------

#[test]
fn removing_an_existing_command_soft_deletes_it_and_returns_success_message() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and a command text='CreateWorkUnit' boundedContextId=0 deleted=false
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(
            vec![
                bounded_context(0, "Work Management"),
                command_item(1, "CreateWorkUnit", 0, false),
            ],
            2,
        ),
    );

    // @step When I dispatch remove-command-from-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "commandName": "CreateWorkUnit"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned message is 'Removed command "CreateWorkUnit" from "Work Management" bounded context'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["message"].as_str(),
        Some("Removed command \"CreateWorkUnit\" from \"Work Management\" bounded context")
    );

    // @step And spec/foundation.json on disk shows the CreateWorkUnit command item deleted=true
    let v = read_foundation(tmp.path());
    let cmd = find_command_by_text(&v, "CreateWorkUnit").expect("command must still exist");
    assert_eq!(
        cmd["deleted"].as_bool(),
        Some(true),
        "command must be soft-deleted"
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

    // @step When I dispatch remove-command-from-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "commandName": "CreateWorkUnit"}),
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
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and a command text='CreateWorkUnit' boundedContextId=0
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(
            vec![
                bounded_context(0, "Work Management"),
                command_item(1, "CreateWorkUnit", 0, false),
            ],
            2,
        ),
    );
    let pre_bytes = fs::read(tmp.path().join("spec/foundation.json")).unwrap();

    // @step When I dispatch remove-command-from-foundation with contextName='Nope' and commandName='CreateWorkUnit'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Nope", "commandName": "CreateWorkUnit"}),
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
fn removing_a_command_name_not_in_matched_context_fails() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and a command text='CreateWorkUnit' boundedContextId=0
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(
            vec![
                bounded_context(0, "Work Management"),
                command_item(1, "CreateWorkUnit", 0, false),
            ],
            2,
        ),
    );
    let pre_bytes = fs::read(tmp.path().join("spec/foundation.json")).unwrap();

    // @step When I dispatch remove-command-from-foundation with contextName='Work Management' and commandName='Ghost'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "commandName": "Ghost"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Command 'Ghost' not found in bounded context 'Work Management'"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Command 'Ghost' not found in bounded context 'Work Management'"),
        "expected missing-command message; got: {err}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/foundation.json")).unwrap();
    assert_eq!(
        pre_bytes, post_bytes,
        "foundation.json must NOT be mutated on failure"
    );
}

#[test]
fn command_belonging_to_a_different_context_id_is_not_matched() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has bounded_context text='Work Management' id=0, bounded_context text='Specification' id=1, and a command text='CreateFeature' boundedContextId=1
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(
            vec![
                bounded_context(0, "Work Management"),
                bounded_context(1, "Specification"),
                command_item(2, "CreateFeature", 1, false),
            ],
            3,
        ),
    );

    // @step When I dispatch remove-command-from-foundation with contextName='Work Management' and commandName='CreateFeature'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "commandName": "CreateFeature"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Command 'CreateFeature' not found in bounded context 'Work Management'"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Command 'CreateFeature' not found in bounded context 'Work Management'"),
        "expected context-mismatch message; got: {err}"
    );
}

#[test]
fn removing_an_already_soft_deleted_command_fails_as_not_found() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and a command text='CreateWorkUnit' boundedContextId=0 deleted=true
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_items(
            vec![
                bounded_context(0, "Work Management"),
                command_item(1, "CreateWorkUnit", 0, true),
            ],
            2,
        ),
    );

    // @step When I dispatch remove-command-from-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "commandName": "CreateWorkUnit"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Command 'CreateWorkUnit' not found in bounded context 'Work Management'"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Command 'CreateWorkUnit' not found in bounded context 'Work Management'"),
        "expected not-found-for-deleted message; got: {err}"
    );
}

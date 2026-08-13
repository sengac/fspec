#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-command-to-foundation-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `add-command-to-foundation` (RPC-175). Each scenario maps to exactly one
// #[test] function with @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-command-to-foundation".to_string(),
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

/// Return the first non-deleted command item in eventStorm.items.
fn first_command(foundation: &Value) -> Option<&Value> {
    foundation["eventStorm"]["items"]
        .as_array()?
        .iter()
        .find(|i| i["type"].as_str() == Some("command"))
}

// ---------- scenarios ----------

#[test]
fn adding_a_command_appends_the_item_and_increments_next_item_id() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_contexts(&[(0, "Work Management")], 1),
    );

    // @step When I dispatch add-command-to-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "commandName": "CreateWorkUnit"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned message is 'Added command "CreateWorkUnit" to "Work Management" bounded context'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["message"].as_str(),
        Some("Added command \"CreateWorkUnit\" to \"Work Management\" bounded context")
    );

    // @step And spec/foundation.json on disk shows eventStorm.nextItemId=2
    let v = read_foundation(tmp.path());
    assert_eq!(v["eventStorm"]["nextItemId"].as_u64(), Some(2));

    // @step And spec/foundation.json on disk shows the appended item has type='command', text='CreateWorkUnit', boundedContextId=0, id=1, deleted=false
    let cmd = first_command(&v).expect("a command item must exist");
    assert_eq!(cmd["type"].as_str(), Some("command"));
    assert_eq!(cmd["text"].as_str(), Some("CreateWorkUnit"));
    assert_eq!(cmd["boundedContextId"].as_u64(), Some(0));
    assert_eq!(cmd["id"].as_u64(), Some(1));
    assert_eq!(cmd["deleted"].as_bool(), Some(false));

    // @step And the appended item createdAt is a fresh ISO-8601 timestamp
    let created = cmd["createdAt"].as_str().expect("createdAt string");
    assert!(
        created.len() == 24 && created.ends_with('Z'),
        "got: {created}"
    );
}

#[test]
fn color_field_is_persisted_as_the_json_string_blue() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_contexts(&[(0, "Work Management")], 1),
    );

    // @step When I dispatch add-command-to-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "commandName": "CreateWorkUnit"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/foundation.json on disk shows the appended item color='blue' (a JSON string, not null)
    let v = read_foundation(tmp.path());
    let cmd = first_command(&v).expect("a command item must exist");
    assert_eq!(
        cmd["color"].as_str(),
        Some("blue"),
        "color must be the JSON string 'blue'"
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

    // @step When I dispatch add-command-to-foundation with contextName='Work Management', commandName='CreateWorkUnit', description='Creates a work unit'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "contextName": "Work Management",
            "commandName": "CreateWorkUnit",
            "description": "Creates a work unit"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/foundation.json on disk shows the appended item description='Creates a work unit'
    let v = read_foundation(tmp.path());
    let cmd = first_command(&v).expect("a command item must exist");
    assert_eq!(cmd["description"].as_str(), Some("Creates a work unit"));

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
    // Find the command object's slice (after the last bounded_context). The
    // command item is the one carrying "boundedContextId", so we anchor the
    // search from that key's position.
    let anchor = raw
        .find("\"boundedContextId\"")
        .expect("command item with boundedContextId must be serialized");
    // Walk backward to the start of the command item object.
    let obj_start = raw[..anchor].rfind('{').expect("command object open brace");
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

    // @step When I dispatch add-command-to-foundation with contextName='Nope' and commandName='CreateWorkUnit'
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
fn command_links_only_to_matching_context_and_second_add_increments_next_item_id() {
    // @step Given a project root tempdir with spec/foundation.json whose eventStorm has bounded_context text='Work Management' id=0 and bounded_context text='Specification' id=1 and nextItemId=2
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(
        tmp.path(),
        &foundation_with_contexts(&[(0, "Work Management"), (1, "Specification")], 2),
    );

    // @step When I dispatch add-command-to-foundation with contextName='Specification' and commandName='CreateFeature'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Specification", "commandName": "CreateFeature"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the appended item has boundedContextId=1 and id=2
    let v = read_foundation(tmp.path());
    let create_feature = v["eventStorm"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["text"].as_str() == Some("CreateFeature"))
        .expect("CreateFeature command must exist");
    assert_eq!(create_feature["boundedContextId"].as_u64(), Some(1));
    assert_eq!(create_feature["id"].as_u64(), Some(2));

    // @step When I dispatch add-command-to-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    let result2 = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "commandName": "CreateWorkUnit"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result2.success, "expected success=true, got {result2:?}");

    // @step And the second appended item has boundedContextId=0 and id=3
    let v2 = read_foundation(tmp.path());
    let create_wu = v2["eventStorm"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["text"].as_str() == Some("CreateWorkUnit"))
        .expect("CreateWorkUnit command must exist");
    assert_eq!(create_wu["boundedContextId"].as_u64(), Some(0));
    assert_eq!(create_wu["id"].as_u64(), Some(3));

    // @step And spec/foundation.json on disk shows eventStorm.nextItemId=4
    assert_eq!(v2["eventStorm"]["nextItemId"].as_u64(), Some(4));

    // @step And both command items are present in eventStorm.items
    let cmd_count = v2["eventStorm"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["type"].as_str() == Some("command"))
        .count();
    assert_eq!(cmd_count, 2, "both command items must be present");
}

#[test]
fn foundation_with_no_event_storm_reports_canonical_not_found_error() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-command-to-foundation with contextName='Work Management' and commandName='CreateWorkUnit'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"contextName": "Work Management", "commandName": "CreateWorkUnit"}),
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

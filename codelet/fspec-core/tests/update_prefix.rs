#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/update-prefix-rust-port.feature
//
// Dispatcher-level acceptance tests for the Rust port of `update-prefix`
// (RPC-313). Each scenario maps to exactly one #[test] function with
// `@step` comments mirroring the Gherkin steps verbatim.
//
// At the end of Phase B these tests MUST fail with `NotYetPorted` because
// the supervisor has not yet wired the dispatcher to call
// `commands::update_prefix::run`. After Phase C + supervisor wiring they
// turn green.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "update-prefix".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

fn write_file(path: &Path, raw: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(path, raw).expect("write file");
}

fn write_prefixes(project_root: &Path, raw: &str) {
    write_file(&project_root.join("spec/prefixes.json"), raw);
}

fn write_epics(project_root: &Path, raw: &str) {
    write_file(&project_root.join("spec/epics.json"), raw);
}

fn read_prefixes_raw(project_root: &Path) -> String {
    fs::read_to_string(project_root.join("spec/prefixes.json"))
        .expect("read spec/prefixes.json")
}

const AUTH_DESC_OLD: &str = r#"{
  "prefixes": {
    "AUTH": {
      "prefix": "AUTH",
      "description": "old",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#;

const AUTH_WITH_EPIC_ID: &str = r#"{
  "prefixes": {
    "AUTH": {
      "prefix": "AUTH",
      "description": "old",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "epicId": "auth-epic"
    }
  }
}"#;

const AUTH_UI_API: &str = r#"{
  "prefixes": {
    "AUTH": {
      "prefix": "AUTH",
      "description": "Auth features",
      "createdAt": "2026-06-01T00:00:00.000Z"
    },
    "UI": {
      "prefix": "UI",
      "description": "User interface",
      "createdAt": "2026-06-02T00:00:00.000Z"
    },
    "API": {
      "prefix": "API",
      "description": "API endpoints",
      "createdAt": "2026-06-03T00:00:00.000Z"
    }
  }
}"#;

const EPICS_AUTH_EPIC: &str = r#"{
  "epics": {
    "auth-epic": {
      "id": "auth-epic",
      "name": "Authentication"
    }
  }
}"#;

/// Cheap ISO-8601 UTC shape check: `YYYY-MM-DDTHH:MM:SS.sssZ` (24 bytes).
/// Matches TS `new Date().toISOString()` byte-for-byte — millisecond
/// fraction is any three digits (was hard-coded `000` under the prior
/// per-command second-precision helpers).
fn iso_8601_shape_ok(s: &str) -> bool {
    if s.len() != 24 {
        return false;
    }
    let bytes = s.as_bytes();
    let digit = |i: usize| bytes[i].is_ascii_digit();
    digit(0)
        && digit(1)
        && digit(2)
        && digit(3)
        && bytes[4] == b'-'
        && digit(5)
        && digit(6)
        && bytes[7] == b'-'
        && digit(8)
        && digit(9)
        && bytes[10] == b'T'
        && digit(11)
        && digit(12)
        && bytes[13] == b':'
        && digit(14)
        && digit(15)
        && bytes[16] == b':'
        && digit(17)
        && digit(18)
        && bytes[19] == b'.'
        && digit(20)
        && digit(21)
        && digit(22)
        && bytes[23] == b'Z'
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_updates_description_on_existing_prefix() {
    // Scenario: Dispatcher updates description on an existing prefix

    // @step Given spec/prefixes.json contains AUTH with description 'old' and a createdAt timestamp
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), AUTH_DESC_OLD);

    // @step When I dispatch update-prefix with args prefix='AUTH' and description='new'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "AUTH", "description": "new" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the returned JSON parses to an object whose root has field success=true
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step Then spec/prefixes.json now has AUTH.description equal to 'new'
    let on_disk: Value =
        serde_json::from_str(&read_prefixes_raw(tmp.path())).expect("parse on-disk");
    let auth = &on_disk["prefixes"]["AUTH"];
    assert_eq!(auth["description"].as_str(), Some("new"));

    // @step Then spec/prefixes.json now has AUTH.updatedAt set to a non-empty ISO-8601 UTC timestamp
    let updated_at = auth["updatedAt"].as_str().expect("updatedAt present");
    assert!(!updated_at.is_empty(), "updatedAt must be non-empty");
    assert!(
        iso_8601_shape_ok(updated_at),
        "updatedAt does not match ISO-8601 UTC shape; got: {updated_at}"
    );

    // @step Then AUTH.createdAt is preserved verbatim from the pre-call value
    assert_eq!(
        auth["createdAt"].as_str(),
        Some("2026-06-01T00:00:00.000Z"),
        "createdAt must be preserved verbatim"
    );
}

#[test]
fn dispatcher_updates_epic_id_when_epic_exists() {
    // Scenario: Dispatcher updates epicId on an existing prefix when the epic exists

    // @step Given spec/prefixes.json contains AUTH with description 'Auth features'
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), AUTH_DESC_OLD);

    // @step Given spec/epics.json contains an epic with id 'auth-epic'
    write_epics(tmp.path(), EPICS_AUTH_EPIC);

    // @step When I dispatch update-prefix with args prefix='AUTH' and epicId='auth-epic'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "AUTH", "epicId": "auth-epic" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then spec/prefixes.json now has AUTH.epicId equal to 'auth-epic'
    let on_disk: Value =
        serde_json::from_str(&read_prefixes_raw(tmp.path())).expect("parse on-disk");
    let auth = &on_disk["prefixes"]["AUTH"];
    assert_eq!(auth["epicId"].as_str(), Some("auth-epic"));

    // @step Then spec/prefixes.json now has AUTH.updatedAt set to a non-empty ISO-8601 UTC timestamp
    let updated_at = auth["updatedAt"].as_str().expect("updatedAt present");
    assert!(!updated_at.is_empty(), "updatedAt must be non-empty");
    assert!(iso_8601_shape_ok(updated_at), "updatedAt shape: {updated_at}");
}

#[test]
fn dispatcher_rejects_unknown_prefix_and_leaves_file_untouched() {
    // Scenario: Dispatcher rejects unknown prefix and leaves the file untouched

    // @step Given spec/prefixes.json is empty (no prefixes registered)
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), r#"{"prefixes":{}}"#);
    let before = read_prefixes_raw(tmp.path());

    // @step When I dispatch update-prefix with args prefix='NONE' and description='ignored'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "NONE", "description": "ignored" }),
    ));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to update prefix'
    assert!(!result.success, "expected success=false; got {result:?}");
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Failed to update prefix"),
        "error must contain 'Failed to update prefix'; got: {msg}"
    );

    // @step Then the error message contains the substring 'Prefix NONE not found'
    assert!(
        msg.contains("Prefix NONE not found"),
        "error must mention 'Prefix NONE not found'; got: {msg}"
    );

    // @step Then spec/prefixes.json is byte-identical to its pre-call content
    let after = read_prefixes_raw(tmp.path());
    assert_eq!(before, after, "file must be untouched on unknown prefix");
}

#[test]
fn dispatcher_rejects_unknown_epic_id_and_leaves_prefixes_untouched() {
    // Scenario: Dispatcher rejects unknown epicId and leaves prefixes.json untouched

    // @step Given spec/prefixes.json contains AUTH with description 'Auth features'
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), AUTH_DESC_OLD);
    let before = read_prefixes_raw(tmp.path());

    // @step Given spec/epics.json does not exist
    assert!(!tmp.path().join("spec/epics.json").exists());

    // @step When I dispatch update-prefix with args prefix='AUTH' and epicId='ghost'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "AUTH", "epicId": "ghost" }),
    ));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to update prefix'
    assert!(!result.success, "expected success=false; got {result:?}");
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Failed to update prefix"),
        "error must contain 'Failed to update prefix'; got: {msg}"
    );

    // @step Then the error message contains the substring 'Epic ghost not found'
    assert!(
        msg.contains("Epic ghost not found"),
        "error must mention 'Epic ghost not found'; got: {msg}"
    );

    // @step Then spec/prefixes.json is byte-identical to its pre-call content
    let after = read_prefixes_raw(tmp.path());
    assert_eq!(before, after, "prefixes.json must be untouched on epic error");
}

#[test]
fn dispatcher_updates_both_description_and_epic_id_in_one_call() {
    // Scenario: Dispatcher updates both description and epicId in one call

    // @step Given spec/prefixes.json contains AUTH with description 'old'
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), AUTH_DESC_OLD);

    // @step Given spec/epics.json contains an epic with id 'auth-epic'
    write_epics(tmp.path(), EPICS_AUTH_EPIC);

    // @step When I dispatch update-prefix with args prefix='AUTH', description='new', epicId='auth-epic'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "prefix": "AUTH",
            "description": "new",
            "epicId": "auth-epic"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then spec/prefixes.json now has AUTH.description equal to 'new'
    let on_disk: Value =
        serde_json::from_str(&read_prefixes_raw(tmp.path())).expect("parse on-disk");
    let auth = &on_disk["prefixes"]["AUTH"];
    assert_eq!(auth["description"].as_str(), Some("new"));

    // @step Then spec/prefixes.json now has AUTH.epicId equal to 'auth-epic'
    assert_eq!(auth["epicId"].as_str(), Some("auth-epic"));

    // @step Then spec/prefixes.json now has AUTH.updatedAt set to a non-empty ISO-8601 UTC timestamp
    let updated_at = auth["updatedAt"].as_str().expect("updatedAt present");
    assert!(!updated_at.is_empty());
    assert!(iso_8601_shape_ok(updated_at));
}

#[test]
fn dispatcher_no_op_bumps_updated_at_preserving_other_fields() {
    // Scenario: Dispatcher no-op bumps updatedAt while preserving description and epicId

    // @step Given spec/prefixes.json contains AUTH with description 'old' and epicId 'auth-epic'
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), AUTH_WITH_EPIC_ID);

    // @step When I dispatch update-prefix with args prefix='AUTH' (no description, no epicId)
    let result = dispatch_command(req(tmp.path(), json!({ "prefix": "AUTH" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then spec/prefixes.json AUTH.description is preserved verbatim as 'old'
    let on_disk: Value =
        serde_json::from_str(&read_prefixes_raw(tmp.path())).expect("parse on-disk");
    let auth = &on_disk["prefixes"]["AUTH"];
    assert_eq!(auth["description"].as_str(), Some("old"));

    // @step Then spec/prefixes.json AUTH.epicId is preserved verbatim as 'auth-epic'
    assert_eq!(auth["epicId"].as_str(), Some("auth-epic"));

    // @step Then spec/prefixes.json AUTH.updatedAt is set to a non-empty ISO-8601 UTC timestamp
    let updated_at = auth["updatedAt"].as_str().expect("updatedAt present");
    assert!(!updated_at.is_empty());
    assert!(iso_8601_shape_ok(updated_at));
}

#[test]
fn dispatcher_preserves_insertion_order_on_non_terminal_update() {
    // Scenario: Dispatcher preserves insertion order when updating a non-terminal entry

    // @step Given spec/prefixes.json contains AUTH then UI then API in that registration order
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), AUTH_UI_API);

    // @step When I dispatch update-prefix with args prefix='AUTH' and description='new'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "AUTH", "description": "new" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then in the on-disk JSON the AUTH entry still appears before the UI entry
    let raw = read_prefixes_raw(tmp.path());
    let auth_pos = raw.find("\"AUTH\"").expect("AUTH key present");
    let ui_pos = raw.find("\"UI\"").expect("UI key present");
    let api_pos = raw.find("\"API\"").expect("API key present");
    assert!(
        auth_pos < ui_pos,
        "AUTH must appear before UI; AUTH={auth_pos} UI={ui_pos}\n{raw}"
    );

    // @step Then in the on-disk JSON the UI entry still appears before the API entry
    assert!(
        ui_pos < api_pos,
        "UI must appear before API; UI={ui_pos} API={api_pos}\n{raw}"
    );
}

#[test]
fn dispatcher_escalates_malformed_prefixes_json() {
    // Scenario: Dispatcher escalates malformed prefixes.json

    // @step Given spec/prefixes.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), "{ not json");
    let before = read_prefixes_raw(tmp.path());

    // @step When I dispatch update-prefix with args prefix='AUTH' and description='new'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "AUTH", "description": "new" }),
    ));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse prefixes.json'
    assert!(!result.success, "expected success=false; got {result:?}");
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Failed to parse prefixes.json"),
        "error must mention 'Failed to parse prefixes.json'; got: {msg}"
    );

    // @step Then spec/prefixes.json is byte-identical to its pre-call content
    let after = read_prefixes_raw(tmp.path());
    assert_eq!(before, after, "malformed file must not be overwritten");
}

#[test]
fn dispatcher_returns_canonical_json_success_shape() {
    // Scenario: Dispatcher returns the canonical JSON success shape

    // @step Given spec/prefixes.json contains AUTH with description 'Auth features'
    let tmp = TempDir::new().expect("tempdir");
    write_prefixes(tmp.path(), AUTH_DESC_OLD);

    // @step When I dispatch update-prefix with args prefix='AUTH' and description='Updated'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "prefix": "AUTH", "description": "Updated" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root object has exactly one field, success=true
    let data = parse_data(&result.data);
    let obj = data.as_object().expect("data is a JSON object");
    assert_eq!(
        obj.len(),
        1,
        "result must have exactly one top-level field; got {obj:?}"
    );
    assert_eq!(obj.get("success").and_then(Value::as_bool), Some(true));
}

#[test]
fn shared_infrastructure_is_reused_without_duplication() {
    // Scenario: Shared infrastructure is reused without duplication

    // @step Given the codelet/fspec-core crate is built
    // (precondition — this test only runs when the crate compiles)

    // @step When I inspect codelet/fspec-core/src/commands/update_prefix.rs
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands/update_prefix.rs");
    let src = fs::read_to_string(&path).expect("read update_prefix.rs");

    // @step Then the source declares it uses `ensure_prefixes_file`, `read_epics_or_empty`, and `write_json_atomic` from the shared io modules
    assert!(
        src.contains("ensure_prefixes_file"),
        "update_prefix.rs must reference `ensure_prefixes_file`; got:\n{src}"
    );
    assert!(
        src.contains("read_epics_or_empty"),
        "update_prefix.rs must reference `read_epics_or_empty`; got:\n{src}"
    );
    assert!(
        src.contains("write_json_atomic"),
        "update_prefix.rs must reference `write_json_atomic`; got:\n{src}"
    );

    // @step Then the source does NOT contain the substring 'FspecCoreError::NotYetPorted'
    assert!(
        !src.contains("FspecCoreError::NotYetPorted"),
        "update_prefix.rs must no longer be a NotYetPorted stub"
    );

    // @step Then the source does NOT inline any std::fs::write or serde_json::to_writer call for spec/prefixes.json
    assert!(
        !src.contains("std::fs::write"),
        "update_prefix.rs must NOT call std::fs::write directly"
    );
    assert!(
        !src.contains("serde_json::to_writer"),
        "update_prefix.rs must NOT call serde_json::to_writer directly"
    );
}

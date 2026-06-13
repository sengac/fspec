#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/remove-architecture-note-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `remove-architecture-note`
// (RPC-267). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-architecture-note".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units_value(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("work-units.json is valid JSON")
}

fn read_work_units_raw(project_root: &Path) -> String {
    fs::read_to_string(project_root.join("spec/work-units.json")).expect("read work-units.json")
}

fn seed_unit_with_notes(project_root: &Path, notes: &str, next_note_id: u32) {
    let body = format!(
        r#"{{
  "version": "0.7.1",
  "meta": {{"version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z"}},
  "workUnits": {{
    "AUTH-001": {{
      "id": "AUTH-001",
      "title": "Login",
      "status": "specifying",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z",
      "architectureNotes": {notes},
      "nextNoteId": {next_note_id}
    }}
  }},
  "states": {{
    "backlog": [], "specifying": ["AUTH-001"], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }}
}}"#
    );
    write_work_units(project_root, &body);
}

// ---------- scenarios ----------

#[test]
fn dispatcher_soft_deletes_the_matching_architecture_note_by_id() {
    // Scenario: Dispatcher soft-deletes the matching architecture note by ID

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with architectureNotes ids 0 and 1
    let tmp = TempDir::new().expect("tempdir");
    seed_unit_with_notes(
        tmp.path(),
        r#"[
        {"id":0,"text":"note zero","deleted":false,"createdAt":"2026-06-01T00:00:00.000Z"},
        {"id":1,"text":"note one","deleted":false,"createdAt":"2026-06-01T00:00:00.000Z"}
      ]"#,
        2,
    );

    // @step When I dispatch remove-architecture-note with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json work unit 'AUTH-001' architectureNotes[0] has deleted=true
    let data = read_work_units_value(tmp.path());
    let notes = data["workUnits"]["AUTH-001"]["architectureNotes"]
        .as_array()
        .expect("notes array");
    assert_eq!(notes[0]["deleted"].as_bool(), Some(true));

    // @step And spec/work-units.json work unit 'AUTH-001' architectureNotes[0] has a non-empty deletedAt
    let deleted_at = notes[0]["deletedAt"]
        .as_str()
        .expect("deletedAt must be set on soft-deleted note");
    assert!(!deleted_at.is_empty(), "deletedAt must be non-empty");

    // @step And spec/work-units.json work unit 'AUTH-001' architectureNotes[1] still has deleted=false
    assert_eq!(notes[1]["deleted"].as_bool(), Some(false));
}

#[test]
fn dispatcher_is_idempotent_on_already_deleted_notes() {
    // Scenario: Dispatcher is idempotent on already-deleted notes

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with architectureNote id=0 already deleted
    let tmp = TempDir::new().expect("tempdir");
    seed_unit_with_notes(
        tmp.path(),
        r#"[
        {"id":0,"text":"already gone","deleted":true,"createdAt":"2026-06-01T00:00:00.000Z","deletedAt":"2026-06-01T00:00:00.000Z"}
      ]"#,
        1,
    );

    // @step When I capture the exact byte contents of spec/work-units.json
    let before = read_work_units_raw(tmp.path());

    // @step And I dispatch remove-architecture-note with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "expected idempotent success; got {result:?}"
    );

    // @step And the DispatchResult.data contains the substring 'Item ID 0 already deleted'
    assert!(
        result.data.contains("Item ID 0 already deleted"),
        "missing idempotent message; got:\n{}",
        result.data
    );

    // @step And spec/work-units.json is byte-equal to the previously captured contents
    let after = read_work_units_raw(tmp.path());
    assert_eq!(
        before, after,
        "idempotent path must NOT mutate disk; before:\n{before}\n\nafter:\n{after}"
    );
}

#[test]
fn dispatcher_rejects_missing_work_unit_ids() {
    // Scenario: Dispatcher rejects missing work unit IDs

    // @step Given spec/work-units.json contains no work unit 'MISSING-001'
    let tmp = TempDir::new().expect("tempdir");
    seed_unit_with_notes(
        tmp.path(),
        r#"[{"id":0,"text":"x","deleted":false,"createdAt":"2026-06-01T00:00:00.000Z"}]"#,
        1,
    );

    // @step When I dispatch remove-architecture-note with workUnitId='MISSING-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "MISSING-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Work unit 'MISSING-001' does not exist"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Work unit 'MISSING-001' does not exist"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_rejects_when_architecture_notes_is_missing_or_empty() {
    // Scenario: Dispatcher rejects when architectureNotes is missing or empty

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with no architectureNotes field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        r#"{
  "version": "0.7.1",
  "meta": {"version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z"},
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Login",
      "status": "specifying",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  },
  "states": {
    "backlog": [], "specifying": ["AUTH-001"], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I dispatch remove-architecture-note with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Work unit 'AUTH-001' has no architecture notes"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Work unit 'AUTH-001' has no architecture notes"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_rejects_an_unknown_architecture_note_id() {
    // Scenario: Dispatcher rejects an unknown architecture note ID

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with architectureNotes ids 0 and 2
    let tmp = TempDir::new().expect("tempdir");
    seed_unit_with_notes(
        tmp.path(),
        r#"[
        {"id":0,"text":"a","deleted":false,"createdAt":"2026-06-01T00:00:00.000Z"},
        {"id":2,"text":"c","deleted":false,"createdAt":"2026-06-01T00:00:00.000Z"}
      ]"#,
        3,
    );

    // @step When I dispatch remove-architecture-note with workUnitId='AUTH-001' and index=1
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 1}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring 'Architecture note with ID 1 not found'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Architecture note with ID 1 not found"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_response_data_contains_the_canonical_success_line() {
    // Scenario: Dispatcher response data contains the canonical success line

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with architectureNote id=0
    let tmp = TempDir::new().expect("tempdir");
    seed_unit_with_notes(
        tmp.path(),
        r#"[{"id":0,"text":"x","deleted":false,"createdAt":"2026-06-01T00:00:00.000Z"}]"#,
        1,
    );

    // @step When I dispatch remove-architecture-note with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line '✓ Architecture note removed successfully'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "✓ Architecture note removed successfully"),
        "missing checkmark line; got:\n{}",
        result.data
    );
}

#[test]
fn dispatcher_fails_fast_when_required_args_are_missing() {
    // Scenario: Dispatcher fails fast when required args are missing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch remove-architecture-note with no workUnitId field in the args
    let result = dispatch_command(req(tmp.path(), json!({"index": 0})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring 'Invalid args for fspec command remove-architecture-note'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Invalid args for fspec command remove-architecture-note"),
        "missing canonical InvalidArgs prefix; got: {msg}"
    );
}

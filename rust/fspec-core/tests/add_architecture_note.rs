#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-architecture-note-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-architecture-note`
// (RPC-168). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-architecture-note".to_string(),
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
        .expect("read spec/work-units.json");
    serde_json::from_str(&raw).expect("work-units.json is valid JSON")
}

fn seed_with_one_unit(project_root: &Path) {
    write_work_units(
        project_root,
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
}

// ---------- scenarios ----------

#[test]
fn dispatcher_appends_a_new_architecture_note_to_an_existing_work_unit() {
    // Scenario: Dispatcher appends a new architecture note to an existing work unit

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with no architectureNotes
    let tmp = TempDir::new().expect("tempdir");
    seed_with_one_unit(tmp.path());

    // @step When I dispatch add-architecture-note with workUnitId='AUTH-001' and note='Uses bcrypt for password hashing'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "note": "Uses bcrypt for password hashing"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/work-units.json work unit 'AUTH-001' has exactly one architectureNote
    let data = read_work_units_value(tmp.path());
    let notes = data["workUnits"]["AUTH-001"]["architectureNotes"]
        .as_array()
        .expect("architectureNotes must be an array");
    assert_eq!(notes.len(), 1, "expected one note, got {notes:?}");

    // @step And that note has id=0, text='Uses bcrypt for password hashing', and deleted=false
    let note = &notes[0];
    assert_eq!(note["id"].as_u64(), Some(0));
    assert_eq!(
        note["text"].as_str(),
        Some("Uses bcrypt for password hashing")
    );
    assert_eq!(note["deleted"].as_bool(), Some(false));

    // @step And work unit 'AUTH-001' has nextNoteId=1
    assert_eq!(
        data["workUnits"]["AUTH-001"]["nextNoteId"].as_u64(),
        Some(1)
    );
}

#[test]
fn dispatcher_increments_next_note_id_on_each_invocation() {
    // Scenario: Dispatcher increments nextNoteId on each invocation

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with no architectureNotes
    let tmp = TempDir::new().expect("tempdir");
    seed_with_one_unit(tmp.path());

    // @step When I dispatch add-architecture-note with workUnitId='AUTH-001' and note='Note A'
    let r1 = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "note": "Note A"}),
    ));
    assert!(r1.success, "{r1:?}");

    // @step And I dispatch add-architecture-note with workUnitId='AUTH-001' and note='Note B'
    let r2 = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "note": "Note B"}),
    ));
    assert!(r2.success, "{r2:?}");

    // @step Then spec/work-units.json work unit 'AUTH-001' has two architectureNotes
    let data = read_work_units_value(tmp.path());
    let notes = data["workUnits"]["AUTH-001"]["architectureNotes"]
        .as_array()
        .expect("notes array");
    assert_eq!(notes.len(), 2, "expected 2 notes, got {notes:?}");

    // @step And the second architecture note has id=1
    assert_eq!(notes[1]["id"].as_u64(), Some(1));

    // @step And work unit 'AUTH-001' has nextNoteId=2
    assert_eq!(
        data["workUnits"]["AUTH-001"]["nextNoteId"].as_u64(),
        Some(2)
    );
}

#[test]
fn dispatcher_rejects_unknown_work_unit_ids() {
    // Scenario: Dispatcher rejects unknown work unit IDs

    // @step Given spec/work-units.json contains no work unit 'MISSING-001'
    let tmp = TempDir::new().expect("tempdir");
    seed_with_one_unit(tmp.path());

    // @step When I dispatch add-architecture-note with workUnitId='MISSING-001' and note='anything'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "MISSING-001", "note": "anything"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Work unit 'MISSING-001' does not exist"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Work unit 'MISSING-001' does not exist"),
        "missing canonical error text; got: {msg}"
    );
}

#[test]
fn dispatcher_fails_fast_when_required_args_are_missing() {
    // Scenario: Dispatcher fails fast when required args are missing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch add-architecture-note with no workUnitId field in the args
    let result = dispatch_command(req(tmp.path(), json!({"note": "x"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring 'Invalid args for fspec command add-architecture-note'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Invalid args for fspec command add-architecture-note"),
        "missing canonical InvalidArgs prefix; got: {msg}"
    );
}

#[test]
fn dispatcher_response_data_contains_canonical_success_line_and_system_reminder() {
    // Scenario: Dispatcher response data contains the canonical success line and system reminder

    // @step Given spec/work-units.json contains work unit 'AUTH-001'
    let tmp = TempDir::new().expect("tempdir");
    seed_with_one_unit(tmp.path());

    // @step When I dispatch add-architecture-note with workUnitId='AUTH-001' and note='Uses bcrypt'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "note": "Uses bcrypt"}),
    ));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the line '✓ Architecture note added successfully'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "✓ Architecture note added successfully"),
        "missing checkmark line; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the substring '<system-reminder>'
    assert!(
        result.data.contains("<system-reminder>"),
        "missing system-reminder block; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the substring 'ARCHITECTURE NOTE ADDED'
    assert!(
        result.data.contains("ARCHITECTURE NOTE ADDED"),
        "missing reminder header; got:\n{}",
        result.data
    );

    // @step And the DispatchResult.data contains the substring '"Uses bcrypt"'
    assert!(
        result.data.contains("\"Uses bcrypt\""),
        "missing quoted note text in reminder; got:\n{}",
        result.data
    );
}

#[test]
fn dispatcher_preserves_unknown_top_level_fields_on_write() {
    // Scenario: Dispatcher preserves unknown top-level fields on write

    // @step Given spec/work-units.json contains work unit 'AUTH-001' and a top-level 'prefixCounters' object
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
  },
  "prefixCounters": {"AUTH": 1}
}"#,
    );

    // @step When I dispatch add-architecture-note with workUnitId='AUTH-001' and note='note'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "note": "note"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/work-units.json still contains the top-level 'prefixCounters' object
    let data = read_work_units_value(tmp.path());
    assert_eq!(
        data["prefixCounters"]["AUTH"].as_u64(),
        Some(1),
        "prefixCounters must round-trip; got:\n{data}"
    );
}

#[test]
fn dispatcher_auto_creates_work_units_json_when_missing() {
    // Scenario: Dispatcher auto-creates spec/work-units.json when missing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch add-architecture-note with workUnitId='AUTH-001' and note='note'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "note": "note"}),
    ));

    // @step Then the file spec/work-units.json exists
    assert!(
        tmp.path().join("spec/work-units.json").exists(),
        "spec/work-units.json must be auto-created by ensure_work_units_file"
    );

    // @step And the dispatcher returns success=false
    assert!(
        !result.success,
        "expected failure on missing work unit; got {result:?}"
    );

    // @step And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Work unit 'AUTH-001' does not exist"),
        "missing canonical error text; got: {msg}"
    );
}

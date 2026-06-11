#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/restore-architecture-note-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `restore-architecture-note` (RPC-287). Each scenario maps to exactly one
// #[test] function with @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "restore-architecture-note".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/work-units.json"))
        .expect("read spec/work-units.json");
    serde_json::from_str(&raw).expect("work-units.json is valid JSON")
}

fn read_work_units_raw(project_root: &Path) -> Vec<u8> {
    fs::read(project_root.join("spec/work-units.json")).expect("read work-units.json bytes")
}

fn note_by_id(on_disk: &Value, id: u64) -> Option<&Value> {
    on_disk["workUnits"]["AUTH-001"]["architectureNotes"]
        .as_array()
        .and_then(|arr| arr.iter().find(|n| n["id"].as_u64() == Some(id)))
}

/// Build a `spec/work-units.json` with a single AUTH-001 unit at the given
/// status, with optional extra JSON fields spliced into the work unit body.
/// `meta.lastUpdated` defaults to `2026-06-01T00:00:00.000Z` unless an
/// override is passed via `meta_last_updated`.
fn work_units_with(status: &str, meta_last_updated: &str, extras_json: &str) -> String {
    let (backlog, specifying, testing, implementing, validating, done, blocked) = match status {
        "backlog" => ("[\"AUTH-001\"]", "[]", "[]", "[]", "[]", "[]", "[]"),
        "specifying" => ("[]", "[\"AUTH-001\"]", "[]", "[]", "[]", "[]", "[]"),
        "testing" => ("[]", "[]", "[\"AUTH-001\"]", "[]", "[]", "[]", "[]"),
        "implementing" => ("[]", "[]", "[]", "[\"AUTH-001\"]", "[]", "[]", "[]"),
        "validating" => ("[]", "[]", "[]", "[]", "[\"AUTH-001\"]", "[]", "[]"),
        "done" => ("[]", "[]", "[]", "[]", "[]", "[\"AUTH-001\"]", "[]"),
        "blocked" => ("[]", "[]", "[]", "[]", "[]", "[]", "[\"AUTH-001\"]"),
        _ => ("[]", "[]", "[]", "[]", "[]", "[]", "[]"),
    };
    format!(
        r#"{{
  "version": "0.7.1",
  "meta": {{ "version": "1.0.0", "lastUpdated": "{meta_last_updated}" }},
  "workUnits": {{
    "AUTH-001": {{
      "id": "AUTH-001",
      "title": "Test",
      "status": "{status}",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"{maybe_comma}{extras_json}
    }}
  }},
  "states": {{
    "backlog": {backlog}, "specifying": {specifying}, "testing": {testing},
    "implementing": {implementing}, "validating": {validating}, "done": {done},
    "blocked": {blocked}
  }}
}}"#,
        maybe_comma = if extras_json.trim().is_empty() { "" } else { ",\n      " },
        extras_json = extras_json.trim(),
    )
}

// ---------- scenarios ----------

#[test]
fn dispatcher_restores_a_soft_deleted_architecture_note_by_stable_id() {
    // Scenario: Dispatcher restores a soft-deleted architecture note by stable ID

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with architectureNote id=0 text 'Note A' marked deleted with deletedAt '1999-01-01T00:00:00.000Z'
    let tmp = TempDir::new().expect("tempdir");
    let raw = work_units_with(
        "specifying",
        "2026-06-01T00:00:00.000Z",
        r#""architectureNotes": [{ "id": 0, "text": "Note A", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "2026-06-01T00:00:00.000Z" }], "nextNoteId": 1"#,
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the dispatcher output contains restoredNote='Note A'
    let parsed: Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed["restoredNote"].as_str(), Some("Note A"));

    // @step And the dispatcher output contains activeCount=1
    assert_eq!(parsed["activeCount"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk shows the architectureNote with id=0 has deleted=false
    let on_disk = read_work_units(tmp.path());
    let n = note_by_id(&on_disk, 0).expect("note id=0 must exist");
    assert_eq!(n["deleted"].as_bool(), Some(false));

    // @step And spec/work-units.json on disk shows the architectureNote with id=0 has no deletedAt field
    assert!(
        n.get("deletedAt").is_none(),
        "deletedAt field must be removed after restore; got: {n}"
    );
}

#[test]
fn dispatcher_is_idempotent_when_the_note_is_already_active() {
    // Scenario: Dispatcher is idempotent when the note is already active

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with architectureNote id=0 text 'Note A' deleted=false
    let tmp = TempDir::new().expect("tempdir");
    let raw = work_units_with(
        "specifying",
        "2026-06-01T00:00:00.000Z",
        r#""architectureNotes": [{ "id": 0, "text": "Note A", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z" }], "nextNoteId": 1"#,
    );
    write_work_units(tmp.path(), &raw);

    // @step When I capture the exact byte contents of spec/work-units.json
    let before = read_work_units_raw(tmp.path());

    // @step And I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the dispatcher output contains message='Item ID 0 already active'
    let parsed: Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed["message"].as_str(), Some("Item ID 0 already active"));

    // @step And spec/work-units.json is byte-equal to the previously captured contents
    let after = read_work_units_raw(tmp.path());
    assert_eq!(before, after, "idempotent path must not write to disk");
}

#[test]
fn dispatcher_rejects_an_unknown_work_unit() {
    // Scenario: Dispatcher rejects an unknown work unit

    // @step Given spec/work-units.json contains no work unit 'MISSING-001'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with("specifying", "2026-06-01T00:00:00.000Z", ""),
    );

    // @step When I dispatch restore-architecture-note with workUnitId='MISSING-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "MISSING-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring "Work unit 'MISSING-001' does not exist"
    let msg = result.error.as_ref().expect("error set");
    assert!(
        msg.contains("Work unit 'MISSING-001' does not exist"),
        "unexpected error: {msg}"
    );
}

#[test]
fn dispatcher_rejects_when_architecture_notes_is_missing_or_empty() {
    // Scenario: Dispatcher rejects when architectureNotes is missing or empty

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with no architectureNotes field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with("specifying", "2026-06-01T00:00:00.000Z", ""),
    );

    // @step When I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring "Work unit 'AUTH-001' has no architecture notes"
    let msg = result.error.as_ref().expect("error set");
    assert!(
        msg.contains("Work unit 'AUTH-001' has no architecture notes"),
        "unexpected error: {msg}"
    );
}

#[test]
fn dispatcher_rejects_when_the_architecture_note_id_is_not_found() {
    // Scenario: Dispatcher rejects when the architecture note ID is not found

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with one architectureNote id=0 marked deleted
    let tmp = TempDir::new().expect("tempdir");
    let raw = work_units_with(
        "specifying",
        "2026-06-01T00:00:00.000Z",
        r#""architectureNotes": [{ "id": 0, "text": "N", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "x" }]"#,
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=5
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 5}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring 'Architecture note with ID 5 not found'
    let msg = result.error.as_ref().expect("error set");
    assert!(
        msg.contains("Architecture note with ID 5 not found"),
        "unexpected error: {msg}"
    );
}

#[test]
fn dispatcher_computes_active_count_as_number_of_non_deleted_notes_after_restoration() {
    // Scenario: Dispatcher computes activeCount as the number of non-deleted notes after restoration

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with three architectureNotes ids 0, 1, 2 where ids 0 and 1 are deleted and id 2 is active
    let tmp = TempDir::new().expect("tempdir");
    let raw = work_units_with(
        "specifying",
        "2026-06-01T00:00:00.000Z",
        r#""architectureNotes": [
        { "id": 0, "text": "N0", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "x" },
        { "id": 1, "text": "N1", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "x" },
        { "id": 2, "text": "N2", "deleted": false, "createdAt": "x" }
      ], "nextNoteId": 3"#,
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=1
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 1}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the dispatcher output contains activeCount=2
    let parsed: Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed["activeCount"].as_u64(), Some(2));

    // @step And spec/work-units.json on disk shows the architectureNote with id=1 has deleted=false
    let on_disk = read_work_units(tmp.path());
    let n = note_by_id(&on_disk, 1).expect("note id=1 must exist");
    assert_eq!(n["deleted"].as_bool(), Some(false));
}

#[test]
fn dispatcher_allows_restoration_regardless_of_work_unit_status() {
    // Scenario: Dispatcher allows restoration regardless of work unit status

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'done' status with one architectureNote id=0 marked deleted
    let tmp = TempDir::new().expect("tempdir");
    let raw = work_units_with(
        "done",
        "2026-06-01T00:00:00.000Z",
        r#""architectureNotes": [{ "id": 0, "text": "N", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "x" }]"#,
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/work-units.json on disk shows the architectureNote with id=0 has deleted=false
    let on_disk = read_work_units(tmp.path());
    let n = note_by_id(&on_disk, 0).expect("note id=0 must exist");
    assert_eq!(n["deleted"].as_bool(), Some(false));
}

#[test]
fn dispatcher_updates_meta_last_updated_as_well_as_work_unit_updated_at() {
    // Scenario: Dispatcher updates meta.lastUpdated as well as workUnit.updatedAt

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with one architectureNote id=0 marked deleted and meta.lastUpdated '1999-01-01T00:00:00.000Z'
    let tmp = TempDir::new().expect("tempdir");
    let raw = work_units_with(
        "specifying",
        "1999-01-01T00:00:00.000Z",
        r#""architectureNotes": [{ "id": 0, "text": "N", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "x" }]"#,
    );
    // Also explicitly set workUnit.updatedAt to the same sentinel by
    // overwriting the file (work_units_with hard-codes updatedAt to
    // 2026-06-01); we replace it for this scenario specifically.
    let raw = raw.replace(
        "\"updatedAt\": \"2026-06-01T00:00:00.000Z\"",
        "\"updatedAt\": \"1999-01-01T00:00:00.000Z\"",
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch restore-architecture-note with workUnitId='AUTH-001' and index=0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/work-units.json on disk has meta.lastUpdated NOT equal to '1999-01-01T00:00:00.000Z'
    let on_disk = read_work_units(tmp.path());
    let meta_lu = on_disk["meta"]["lastUpdated"]
        .as_str()
        .expect("meta.lastUpdated must be a string");
    assert_ne!(
        meta_lu, "1999-01-01T00:00:00.000Z",
        "meta.lastUpdated must be bumped after restore"
    );

    // @step And spec/work-units.json on disk has workUnits.'AUTH-001'.updatedAt NOT equal to '1999-01-01T00:00:00.000Z'
    let wu_up = on_disk["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("workUnit.updatedAt must be a string");
    assert_ne!(
        wu_up, "1999-01-01T00:00:00.000Z",
        "workUnit.updatedAt must be bumped after restore"
    );
}

#[test]
fn dispatcher_fails_fast_when_required_args_are_missing() {
    // Scenario: Dispatcher fails fast when required args are missing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch restore-architecture-note with no workUnitId field in the args
    let result = dispatch_command(req(tmp.path(), json!({"index": 0})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring 'Invalid args for fspec command restore-architecture-note'
    let msg = result.error.as_ref().expect("error set");
    assert!(
        msg.contains("Invalid args for fspec command restore-architecture-note"),
        "unexpected error: {msg}"
    );
}

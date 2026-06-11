#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/remove-question-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `remove-question`
// (RPC-278). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-question".to_string(),
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

fn question_by_id(on_disk: &Value, id: u64) -> Option<&Value> {
    on_disk["workUnits"]["AUTH-001"]["questions"]
        .as_array()
        .and_then(|arr| arr.iter().find(|q| q["id"].as_u64() == Some(id)))
}

/// Build a `spec/work-units.json` with a single AUTH-001 unit. The
/// `extras` JSON fragment is spliced into the work-unit object so each
/// scenario can pre-populate `questions`, `nextQuestionId`, etc.
fn work_units_with(status: &str, extras_json: &str) -> String {
    // Each status bucket is rendered exactly once below, with the work
    // unit placed into the bucket matching `status`. This avoids any
    // duplicate JSON keys in the `states` object.
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
  "meta": {{ "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" }},
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
fn soft_deletes_a_question_by_stable_id() {
    // Scenario: Soft-deletes a question by stable ID

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 text 'Q?' not deleted
    let tmp = TempDir::new().expect("tempdir");
    let raw = work_units_with(
        "specifying",
        r#""questions": [{ "id": 0, "text": "Q?", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z", "selected": false }], "nextQuestionId": 1"#,
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch remove-question with workUnitId 'AUTH-001' and index 0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the dispatcher output contains removedQuestion='Q?'
    let parsed: Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed["removedQuestion"].as_str(), Some("Q?"));

    // @step And the dispatcher output contains remainingCount=0
    assert_eq!(parsed["remainingCount"].as_u64(), Some(0));

    // @step And spec/work-units.json on disk shows the question with id=0 has deleted=true
    let on_disk = read_work_units(tmp.path());
    let q = question_by_id(&on_disk, 0).expect("question id=0 must exist");
    assert_eq!(q["deleted"].as_bool(), Some(true));

    // @step And spec/work-units.json on disk shows the question with id=0 has a deletedAt timestamp
    let deleted_at = q["deletedAt"].as_str().expect("deletedAt must be set");
    assert!(!deleted_at.is_empty(), "deletedAt must be non-empty");
}

#[test]
fn rejects_remove_question_when_the_work_unit_does_not_exist() {
    // Scenario: Rejects remove-question when the work unit does not exist

    // @step Given spec/work-units.json contains no work unit 'AUTH-999'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("specifying", ""));

    // @step When I dispatch remove-question with workUnitId 'AUTH-999' and index 0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-999", "index": 0}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring "Work unit 'AUTH-999' does not exist"
    let msg = result.error.as_ref().expect("error set");
    assert!(
        msg.contains("Work unit 'AUTH-999' does not exist"),
        "unexpected error: {msg}"
    );
}

#[test]
fn rejects_remove_question_when_the_work_unit_is_not_in_specifying_status() {
    // Scenario: Rejects remove-question when the work unit is not in specifying status

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'testing' status with one question id=0
    let tmp = TempDir::new().expect("tempdir");
    let raw = work_units_with(
        "testing",
        r#""questions": [{ "id": 0, "text": "Q?", "deleted": false, "createdAt": "x", "selected": false }]"#,
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch remove-question with workUnitId 'AUTH-001' and index 0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring "Can only remove questions during discovery/specification phase. AUTH-001 is in 'testing' state."
    let msg = result.error.as_ref().expect("error set");
    assert!(
        msg.contains("Can only remove questions during discovery/specification phase. AUTH-001 is in 'testing' state."),
        "unexpected error: {msg}"
    );
}

#[test]
fn rejects_remove_question_when_the_work_unit_has_no_questions() {
    // Scenario: Rejects remove-question when the work unit has no questions

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with no questions array
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("specifying", ""));

    // @step When I dispatch remove-question with workUnitId 'AUTH-001' and index 0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring 'Work unit AUTH-001 has no questions'
    let msg = result.error.as_ref().expect("error set");
    assert!(
        msg.contains("Work unit AUTH-001 has no questions"),
        "unexpected error: {msg}"
    );
}

#[test]
fn rejects_remove_question_when_the_question_id_is_not_found() {
    // Scenario: Rejects remove-question when the question ID is not found

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0
    let tmp = TempDir::new().expect("tempdir");
    let raw = work_units_with(
        "specifying",
        r#""questions": [{ "id": 0, "text": "Q?", "deleted": false, "createdAt": "x", "selected": false }]"#,
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch remove-question with workUnitId 'AUTH-001' and index 5
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 5}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring 'Question with ID 5 not found'
    let msg = result.error.as_ref().expect("error set");
    assert!(
        msg.contains("Question with ID 5 not found"),
        "unexpected error: {msg}"
    );
}

#[test]
fn returns_idempotent_success_when_the_question_is_already_deleted() {
    // Scenario: Returns idempotent success when the question is already deleted

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 already soft-deleted with deletedAt '1999-01-01T00:00:00.000Z'
    let tmp = TempDir::new().expect("tempdir");
    let raw = work_units_with(
        "specifying",
        r#""questions": [{ "id": 0, "text": "Q?", "deleted": true, "createdAt": "2026-06-01T00:00:00.000Z", "selected": false, "deletedAt": "1999-01-01T00:00:00.000Z" }]"#,
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch remove-question with workUnitId 'AUTH-001' and index 0
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the dispatcher output contains message='Item ID 0 already deleted'
    let parsed: Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed["message"].as_str(), Some("Item ID 0 already deleted"));

    // @step And spec/work-units.json on disk shows the question with id=0 still has deletedAt='1999-01-01T00:00:00.000Z'
    let on_disk = read_work_units(tmp.path());
    let q = question_by_id(&on_disk, 0).expect("question id=0 must exist");
    assert_eq!(
        q["deletedAt"].as_str(),
        Some("1999-01-01T00:00:00.000Z"),
        "deletedAt must be preserved on idempotent path"
    );
}

#[test]
fn counts_only_non_deleted_questions_in_remaining_count() {
    // Scenario: Counts only non-deleted questions in remainingCount

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with three questions ids 0, 1, 2 (none deleted)
    let tmp = TempDir::new().expect("tempdir");
    let raw = work_units_with(
        "specifying",
        r#""questions": [
        { "id": 0, "text": "Q0", "deleted": false, "createdAt": "x", "selected": false },
        { "id": 1, "text": "Q1", "deleted": false, "createdAt": "x", "selected": false },
        { "id": 2, "text": "Q2", "deleted": false, "createdAt": "x", "selected": false }
      ], "nextQuestionId": 3"#,
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch remove-question with workUnitId 'AUTH-001' and index 1
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 1}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the dispatcher output contains remainingCount=2
    let parsed: Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed["remainingCount"].as_u64(), Some(2));

    // @step And spec/work-units.json on disk contains 3 question records
    let on_disk = read_work_units(tmp.path());
    let arr = on_disk["workUnits"]["AUTH-001"]["questions"]
        .as_array()
        .expect("questions array");
    assert_eq!(arr.len(), 3, "soft-delete preserves the array length");

    // @step And spec/work-units.json on disk shows the question with id=1 has deleted=true
    let q = question_by_id(&on_disk, 1).expect("question id=1 must exist");
    assert_eq!(q["deleted"].as_bool(), Some(true));
}

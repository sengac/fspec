#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/add-question-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-question`
// (RPC-188). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-question".to_string(),
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

/// Build a minimal `spec/work-units.json` content with one work unit in
/// the provided status. Returns the raw JSON string ready for
/// `write_work_units`.
fn minimal_work_units(id: &str, status: &str) -> String {
    format!(
        r#"{{
  "version": "0.7.1",
  "meta": {{ "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" }},
  "workUnits": {{
    "{id}": {{
      "id": "{id}",
      "title": "Test unit",
      "status": "{status}",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }}
  }},
  "states": {{
    "backlog": [],
    "specifying": [{specifying}],
    "testing": [],
    "implementing": [],
    "validating": [],
    "done": [],
    "blocked": [{blocked}]
  }}
}}"#,
        id = id,
        status = status,
        specifying = if status == "specifying" { format!("\"{id}\"") } else { String::new() },
        blocked = if status == "blocked" { format!("\"{id}\"") } else { String::new() },
    )
}

// ---------- scenarios ----------

#[test]
fn adds_a_question_with_human_mention_to_a_specifying_work_unit() {
    // Scenario: Adds a question with @human mention to a specifying work unit

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with no questions array
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &minimal_work_units("AUTH-001", "specifying"));

    // @step When I dispatch add-question with workUnitId 'AUTH-001' and question '@human: Support OAuth?'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "question": "@human: Support OAuth?"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the dispatcher output contains questionCount=1
    let parsed: Value = serde_json::from_str(&result.data).expect("data must be JSON");
    assert_eq!(parsed["questionCount"].as_u64(), Some(1));

    // @step And the dispatcher output contains mentionedPeople=['human']
    let mentioned = parsed["mentionedPeople"]
        .as_array()
        .expect("mentionedPeople must be an array");
    assert_eq!(mentioned.len(), 1);
    assert_eq!(mentioned[0].as_str(), Some("human"));

    // @step And spec/work-units.json on disk contains a question with id=0 and text '@human: Support OAuth?'
    let on_disk = read_work_units(tmp.path());
    let questions = on_disk["workUnits"]["AUTH-001"]["questions"]
        .as_array()
        .expect("questions must be an array");
    assert_eq!(questions.len(), 1);
    assert_eq!(questions[0]["id"].as_u64(), Some(0));
    assert_eq!(questions[0]["text"].as_str(), Some("@human: Support OAuth?"));
    assert_eq!(questions[0]["deleted"].as_bool(), Some(false));
    assert_eq!(questions[0]["selected"].as_bool(), Some(false));

    // @step And spec/work-units.json on disk contains nextQuestionId=1 on AUTH-001
    assert_eq!(
        on_disk["workUnits"]["AUTH-001"]["nextQuestionId"].as_u64(),
        Some(1)
    );
}

#[test]
fn rejects_add_question_when_the_work_unit_does_not_exist() {
    // Scenario: Rejects add-question when the work unit does not exist

    // @step Given spec/work-units.json contains no work unit 'AUTH-999'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &minimal_work_units("AUTH-001", "specifying"));

    // @step When I dispatch add-question with workUnitId 'AUTH-999' and question 'Q?'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-999", "question": "Q?"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains the substring "Work unit 'AUTH-999' does not exist"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Work unit 'AUTH-999' does not exist"),
        "unexpected error message: {msg}"
    );
}

#[test]
fn rejects_add_question_when_the_work_unit_is_not_in_specifying_status() {
    // Scenario: Rejects add-question when the work unit is not in specifying status

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'backlog' status
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &minimal_work_units("AUTH-001", "backlog"));

    // @step When I dispatch add-question with workUnitId 'AUTH-001' and question 'Q?'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "question": "Q?"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");

    // @step And the error message contains the substring "Can only add questions during discovery/specification phase. AUTH-001 is in 'backlog' state."
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains(
            "Can only add questions during discovery/specification phase. AUTH-001 is in 'backlog' state."
        ),
        "unexpected error message: {msg}"
    );
}

#[test]
fn honors_existing_next_question_id_by_reusing_it_and_bumping_by_one() {
    // Scenario: Honors existing nextQuestionId by reusing it and bumping by one

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with nextQuestionId=3
    let tmp = TempDir::new().expect("tempdir");
    let raw = format!(
        r#"{{
  "version": "0.7.1",
  "meta": {{ "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" }},
  "workUnits": {{
    "AUTH-001": {{
      "id": "AUTH-001",
      "title": "Test",
      "status": "specifying",
      "nextQuestionId": 3,
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }}
  }},
  "states": {{
    "backlog": [], "specifying": ["AUTH-001"], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }}
}}"#
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch add-question with workUnitId 'AUTH-001' and question 'New question'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "question": "New question"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the dispatcher output contains questionCount=1
    let parsed: Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert_eq!(parsed["questionCount"].as_u64(), Some(1));

    // @step And spec/work-units.json on disk contains a question with id=3
    let on_disk = read_work_units(tmp.path());
    let questions = on_disk["workUnits"]["AUTH-001"]["questions"]
        .as_array()
        .expect("questions array");
    assert_eq!(questions.len(), 1);
    assert_eq!(questions[0]["id"].as_u64(), Some(3));

    // @step And spec/work-units.json on disk contains nextQuestionId=4 on AUTH-001
    assert_eq!(
        on_disk["workUnits"]["AUTH-001"]["nextQuestionId"].as_u64(),
        Some(4)
    );
}

#[test]
fn omits_mentioned_people_when_no_mention_is_present_in_the_question() {
    // Scenario: Omits mentionedPeople when no @mention is present in the question

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &minimal_work_units("AUTH-001", "specifying"));

    // @step When I dispatch add-question with workUnitId 'AUTH-001' and question 'Should we add caching?'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "question": "Should we add caching?"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the dispatcher output does NOT contain the field 'mentionedPeople'
    let parsed: Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert!(
        parsed.get("mentionedPeople").is_none(),
        "mentionedPeople must be omitted when empty; got: {parsed}"
    );
}

#[test]
fn preserves_auxiliary_work_unit_fields_on_round_trip_write() {
    // Scenario: Preserves auxiliary work unit fields on round-trip write

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with auxiliary rules, examples, and virtualHooks arrays
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Test",
      "status": "specifying",
      "rules": [{ "id": 0, "text": "Rule A", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z" }],
      "examples": [{ "id": 0, "text": "Example A", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z" }],
      "virtualHooks": [{ "name": "lint", "event": "pre-implementing", "command": "eslint", "blocking": true }],
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  },
  "states": {
    "backlog": [], "specifying": ["AUTH-001"], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#;
    write_work_units(tmp.path(), raw);

    // @step When I dispatch add-question with workUnitId 'AUTH-001' and question 'Q?'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "question": "Q?"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/work-units.json on disk still contains the original rules array
    let on_disk = read_work_units(tmp.path());
    let rules = on_disk["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .expect("rules array preserved");
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["text"].as_str(), Some("Rule A"));

    // @step And spec/work-units.json on disk still contains the original examples array
    let examples = on_disk["workUnits"]["AUTH-001"]["examples"]
        .as_array()
        .expect("examples array preserved");
    assert_eq!(examples.len(), 1);
    assert_eq!(examples[0]["text"].as_str(), Some("Example A"));

    // @step And spec/work-units.json on disk still contains the original virtualHooks array
    let hooks = on_disk["workUnits"]["AUTH-001"]["virtualHooks"]
        .as_array()
        .expect("virtualHooks array preserved");
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0]["name"].as_str(), Some("lint"));
}

#[test]
fn initializes_missing_next_question_id_to_zero_for_backward_compatibility() {
    // Scenario: Initializes missing nextQuestionId to 0 for backward compatibility

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with no nextQuestionId field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &minimal_work_units("AUTH-001", "specifying"));

    // @step When I dispatch add-question with workUnitId 'AUTH-001' and question 'First question'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "question": "First question"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And spec/work-units.json on disk contains a question with id=0
    let on_disk = read_work_units(tmp.path());
    let questions = on_disk["workUnits"]["AUTH-001"]["questions"]
        .as_array()
        .expect("questions array");
    assert_eq!(questions.len(), 1);
    assert_eq!(questions[0]["id"].as_u64(), Some(0));

    // @step And spec/work-units.json on disk contains nextQuestionId=1 on AUTH-001
    assert_eq!(
        on_disk["workUnits"]["AUTH-001"]["nextQuestionId"].as_u64(),
        Some(1)
    );
}

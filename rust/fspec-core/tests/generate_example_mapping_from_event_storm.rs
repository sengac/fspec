#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/generate-example-mapping-from-event-storm-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `generate-example-mapping-from-event-storm` (RPC-232). Each scenario maps to
// one #[test] fn with @step comments mirroring the Gherkin steps verbatim.
//
// The command transforms a work unit's Event Storm artifacts into Example
// Mapping entries:
//   * policy (when+then) -> rule "System must <then> after <when>"  (pascalCaseToSentence)
//   * event              -> NOTHING (BUG-089: examplesAdded always 0)
//   * hotspot (concern)  -> question "@human: <concern>?"           (BUG-088: trailing ? added only if absent)
// Soft-deleted items are skipped. A missing spec/work-units.json is an ERROR
// (Option B — NOT auto-created). Tests drive the LLM-facing dispatcher; until
// Phase C wiring they fail with NotYetPorted, which is the intended red phase.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "generate-example-mapping-from-event-storm".to_string(),
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
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

/// Seed a single work unit (specifying) carrying the supplied eventStorm
/// `items` array. `meta.lastUpdated` is seeded so timestamp-refresh can be
/// asserted. Returns the full pretty-printable Value.
fn seed_unit_with_event_storm(id: &str, items: Value) -> Value {
    json!({
        "version": "0.7.1",
        "meta": {
            "version": "1.0.0",
            "lastUpdated": "2026-06-01T00:00:00.000Z"
        },
        "workUnits": {
            id: {
                "id": id,
                "title": format!("title {id}"),
                "type": "story",
                "status": "specifying",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z",
                "eventStorm": {
                    "level": "process_modeling",
                    "items": items,
                    "nextItemId": 0
                }
            }
        },
        "states": {
            "backlog": [],
            "specifying": [id],
            "testing": [],
            "implementing": [],
            "validating": [],
            "done": [],
            "blocked": []
        }
    })
}

/// Seed a single specifying work unit WITHOUT any eventStorm field.
fn seed_unit_no_event_storm(id: &str) -> Value {
    json!({
        "version": "0.7.1",
        "workUnits": {
            id: {
                "id": id,
                "title": format!("title {id}"),
                "type": "story",
                "status": "specifying",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": [], "specifying": [id], "testing": [], "implementing": [],
            "validating": [], "done": [], "blocked": []
        }
    })
}

fn policy_item(id: u64, text: &str, when: &str, then: &str, deleted: bool) -> Value {
    json!({
        "id": id, "type": "policy", "color": "purple", "text": text,
        "when": when, "then": then, "deleted": deleted,
        "createdAt": "2026-06-01T00:00:00.000Z"
    })
}

fn hotspot_item(id: u64, text: &str, concern: &str, deleted: bool) -> Value {
    json!({
        "id": id, "type": "hotspot", "color": "red", "text": text,
        "concern": concern, "deleted": deleted,
        "createdAt": "2026-06-01T00:00:00.000Z"
    })
}

fn write_value(project_root: &Path, v: &Value) {
    write_work_units(project_root, &serde_json::to_string_pretty(v).unwrap());
}

/// Active (non-deleted) entries of a named array on the work unit.
fn active_entries<'a>(v: &'a Value, id: &str, field: &str) -> Vec<&'a Value> {
    v["workUnits"][id][field]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|e| !matches!(e.get("deleted"), Some(Value::Bool(true))))
                .collect()
        })
        .unwrap_or_default()
}

// ---------- scenarios ----------

#[test]
fn dispatcher_derives_rules_from_policies_and_questions_from_hotspots() {
    // @step Given spec/work-units.json contains AUTH-001 in specifying status with an eventStorm containing 2 policies (each with when+then) and 1 hotspot with a concern
    let tmp = TempDir::new().expect("tempdir");
    let items = json!([
        policy_item(
            0,
            "Send welcome email",
            "UserRegistered",
            "SendWelcomeEmail",
            false
        ),
        policy_item(
            1,
            "Send verification",
            "UserRegistered",
            "SendVerificationEmail",
            false
        ),
        hotspot_item(2, "Email timeout", "How long to wait", false)
    ]);
    write_value(tmp.path(), &seed_unit_with_event_storm("AUTH-001", items));

    // @step When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success with rulesAdded=2, examplesAdded=0, questionsAdded=1
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["rulesAdded"].as_u64(), Some(2));
    assert_eq!(data["examplesAdded"].as_u64(), Some(0));
    assert_eq!(data["questionsAdded"].as_u64(), Some(1));

    // @step Then the work unit's rules array contains 2 new rule entries
    let v = read_work_units(tmp.path());
    assert_eq!(active_entries(&v, "AUTH-001", "rules").len(), 2);

    // @step Then the work unit's questions array contains 1 new question entry
    assert_eq!(active_entries(&v, "AUTH-001", "questions").len(), 1);

    // @step Then the work unit's examples array remains empty
    assert_eq!(active_entries(&v, "AUTH-001", "examples").len(), 0);
}

#[test]
fn dispatcher_returns_missing_file_error_in_empty_workspace() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success=false with an error message exactly 'spec/work-units.json not found. Run fspec init first.'
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("spec/work-units.json not found. Run fspec init first."),
        "expected TS-parity missing-file message; got: {err}"
    );

    // @step Then spec/work-units.json does NOT exist after the call
    assert!(
        !tmp.path().join("spec/work-units.json").exists(),
        "command must NOT auto-create the file (Option B)"
    );
}

#[test]
fn dispatcher_returns_work_unit_not_found_when_id_absent() {
    // @step Given spec/work-units.json contains BUG-001 but not AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_value(tmp.path(), &seed_unit_no_event_storm("BUG-001"));

    // @step When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 not found'
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit AUTH-001 not found"),
        "expected missing work-unit message; got: {err}"
    );
}

#[test]
fn dispatcher_returns_no_event_storm_data_error() {
    // @step Given spec/work-units.json contains AUTH-001 with no eventStorm field
    let tmp = TempDir::new().expect("tempdir");
    write_value(tmp.path(), &seed_unit_no_event_storm("AUTH-001"));

    // @step When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 has no Event Storm data'
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit AUTH-001 has no Event Storm data"),
        "expected no-event-storm message; got: {err}"
    );
}

#[test]
fn policy_is_converted_to_a_rule_using_pascal_case_to_sentence() {
    // @step Given spec/work-units.json contains AUTH-001 with an eventStorm policy when='UserRegistered' then='SendWelcomeEmail'
    let tmp = TempDir::new().expect("tempdir");
    let items = json!([policy_item(
        0,
        "Send welcome email",
        "UserRegistered",
        "SendWelcomeEmail",
        false
    )]);
    write_value(tmp.path(), &seed_unit_with_event_storm("AUTH-001", items));

    // @step When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then a rule is appended with text exactly 'System must send welcome email after user registered'
    let v = read_work_units(tmp.path());
    let rules = active_entries(&v, "AUTH-001", "rules");
    assert_eq!(rules.len(), 1, "expected exactly one rule");
    assert_eq!(
        rules[0]["text"].as_str(),
        Some("System must send welcome email after user registered")
    );
}

#[test]
fn hotspot_concern_becomes_at_human_question_with_trailing_question_mark_added() {
    // @step Given spec/work-units.json contains AUTH-001 with an eventStorm hotspot concern='Unclear how long to wait'
    let tmp = TempDir::new().expect("tempdir");
    let items = json!([hotspot_item(
        0,
        "Email timeout",
        "Unclear how long to wait",
        false
    )]);
    write_value(tmp.path(), &seed_unit_with_event_storm("AUTH-001", items));

    // @step When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then a question is appended with text exactly '@human: Unclear how long to wait?'
    let v = read_work_units(tmp.path());
    let questions = active_entries(&v, "AUTH-001", "questions");
    assert_eq!(questions.len(), 1, "expected exactly one question");
    assert_eq!(
        questions[0]["text"].as_str(),
        Some("@human: Unclear how long to wait?")
    );
}

#[test]
fn hotspot_concern_already_ending_in_question_mark_is_preserved() {
    // @step Given spec/work-units.json contains AUTH-001 with an eventStorm hotspot concern='How long to wait?'
    let tmp = TempDir::new().expect("tempdir");
    let items = json!([hotspot_item(0, "Email timeout", "How long to wait?", false)]);
    write_value(tmp.path(), &seed_unit_with_event_storm("AUTH-001", items));

    // @step When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then a question is appended with text exactly '@human: How long to wait?'
    let v = read_work_units(tmp.path());
    let questions = active_entries(&v, "AUTH-001", "questions");
    assert_eq!(questions.len(), 1, "expected exactly one question");
    assert_eq!(
        questions[0]["text"].as_str(),
        Some("@human: How long to wait?")
    );
}

#[test]
fn soft_deleted_event_storm_items_are_skipped() {
    // @step Given spec/work-units.json contains AUTH-001 with an eventStorm where the only policy and the only hotspot are marked deleted:true
    let tmp = TempDir::new().expect("tempdir");
    let items = json!([
        policy_item(
            0,
            "Send welcome email",
            "UserRegistered",
            "SendWelcomeEmail",
            true
        ),
        hotspot_item(1, "Email timeout", "How long to wait", true)
    ]);
    write_value(tmp.path(), &seed_unit_with_event_storm("AUTH-001", items));

    // @step When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success with rulesAdded=0, examplesAdded=0, questionsAdded=0
    assert!(result.success, "expected success=true, got {result:?}");
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["rulesAdded"].as_u64(), Some(0));
    assert_eq!(data["examplesAdded"].as_u64(), Some(0));
    assert_eq!(data["questionsAdded"].as_u64(), Some(0));
}

#[test]
fn successful_run_bumps_timestamps_and_persists_atomically() {
    // @step Given spec/work-units.json contains AUTH-001 with an eventStorm containing 1 policy with when+then
    let tmp = TempDir::new().expect("tempdir");
    let items = json!([policy_item(
        0,
        "Send welcome email",
        "UserRegistered",
        "SendWelcomeEmail",
        false
    )]);
    write_value(tmp.path(), &seed_unit_with_event_storm("AUTH-001", items));
    let seeded_ts = "2026-06-01T00:00:00.000Z";

    // @step When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the work unit's updatedAt is refreshed to a new ISO-8601 timestamp
    let v = read_work_units(tmp.path());
    let updated_at = v["workUnits"]["AUTH-001"]["updatedAt"].as_str().unwrap();
    assert_ne!(updated_at, seeded_ts, "updatedAt must be refreshed");
    assert!(updated_at.ends_with('Z'), "updatedAt must be ISO-8601 Zulu");

    // @step Then the file meta.lastUpdated is refreshed to a new ISO-8601 timestamp
    let last_updated = v["meta"]["lastUpdated"].as_str().unwrap();
    assert_ne!(
        last_updated, seeded_ts,
        "meta.lastUpdated must be refreshed"
    );
    assert!(
        last_updated.ends_with('Z'),
        "meta.lastUpdated must be ISO-8601 Zulu"
    );
}

// ---------- scenarios moved from the BUG-088, BUG-089 and duplicate question id features (COV-072) ----------

fn event_item(id: u64, text: &str) -> Value {
    json!({
        "id": id, "type": "event", "color": "orange", "text": text,
        "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z"
    })
}

/// The ids of the work unit's questions, in array order.
fn question_ids(v: &Value, id: &str) -> Vec<u64> {
    v["workUnits"][id]["questions"]
        .as_array()
        .map(|a| a.iter().filter_map(|q| q["id"].as_u64()).collect())
        .unwrap_or_default()
}

#[test]
fn domain_events_add_no_examples_while_policies_and_hotspots_are_transformed() {
    // @step Given spec/work-units.json contains AUTH-001 with an eventStorm holding the event 'TrackPlayed', the policy when='UserAuthenticated' then='LoadDashboard' and the hotspot concern='What happens if session expires?'
    let tmp = TempDir::new().expect("tempdir");
    let items = json!([
        policy_item(
            0,
            "Load dashboard",
            "UserAuthenticated",
            "LoadDashboard",
            false
        ),
        hotspot_item(1, "Session", "What happens if session expires?", false),
        event_item(2, "TrackPlayed")
    ]);
    write_value(tmp.path(), &seed_unit_with_event_storm("AUTH-001", items));

    // @step When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the dispatcher returns success with rulesAdded=1, examplesAdded=0, questionsAdded=1
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["rulesAdded"].as_u64(), Some(1), "got {data}");
    assert_eq!(data["examplesAdded"].as_u64(), Some(0), "got {data}");
    assert_eq!(data["questionsAdded"].as_u64(), Some(1), "got {data}");

    // @step And the work unit's examples array is empty
    let v = read_work_units(tmp.path());
    assert_eq!(
        v["workUnits"]["AUTH-001"]["examples"],
        json!([]),
        "events must not become examples; got {v}"
    );
}

#[test]
fn hotspots_get_sequential_question_ids_from_an_undefined_next_question_id() {
    // @step Given spec/work-units.json contains AUTH-001 with no questions and no nextQuestionId and an eventStorm with 3 hotspots
    let tmp = TempDir::new().expect("tempdir");
    let items = json!([
        hotspot_item(0, "One", "First concern", false),
        hotspot_item(1, "Two", "Second concern", false),
        hotspot_item(2, "Three", "Third concern", false)
    ]);
    let seed = seed_unit_with_event_storm("AUTH-001", items);
    assert!(seed["workUnits"]["AUTH-001"]
        .get("nextQuestionId")
        .is_none());
    assert!(seed["workUnits"]["AUTH-001"].get("questions").is_none());
    write_value(tmp.path(), &seed);

    // @step When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the 3 questions have the ids 0, 1 and 2
    let v = read_work_units(tmp.path());
    assert_eq!(question_ids(&v, "AUTH-001"), vec![0, 1, 2], "got {v}");

    // @step And the work unit's nextQuestionId is 3
    assert_eq!(
        v["workUnits"]["AUTH-001"]["nextQuestionId"].as_u64(),
        Some(3),
        "got {v}"
    );
}

#[test]
fn a_second_run_and_add_question_continue_the_question_ids_without_duplicates() {
    // @step Given spec/work-units.json contains AUTH-001 with an eventStorm with 2 hotspots
    let tmp = TempDir::new().expect("tempdir");
    let items = json!([
        hotspot_item(0, "One", "First concern", false),
        hotspot_item(1, "Two", "Second concern", false)
    ]);
    write_value(tmp.path(), &seed_unit_with_event_storm("AUTH-001", items));

    // @step And generate-example-mapping-from-event-storm has run once for AUTH-001
    let first = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));
    assert!(
        first.success,
        "first run: expected success=true, got {first:?}"
    );

    // @step When a third hotspot is added and generate-example-mapping-from-event-storm runs again
    let mut v = read_work_units(tmp.path());
    v["workUnits"]["AUTH-001"]["eventStorm"]["items"]
        .as_array_mut()
        .expect("items array")
        .push(hotspot_item(2, "Three", "Third concern", false));
    write_value(tmp.path(), &v);
    let second = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));
    assert!(
        second.success,
        "second run: expected success=true, got {second:?}"
    );

    // @step And I dispatch add-question with workUnitId='AUTH-001' and question='@human: Anything else?'
    let added = dispatch_command(DispatchRequest {
        command: "add-question".to_string(),
        args_json: json!({"workUnitId": "AUTH-001", "question": "@human: Anything else?"})
            .to_string(),
        project_root: tmp.path().to_path_buf(),
    });
    assert!(
        added.success,
        "add-question: expected success=true, got {added:?}"
    );

    // @step Then the question ids are sequential from 0 with no duplicates
    let v = read_work_units(tmp.path());
    let ids = question_ids(&v, "AUTH-001");
    let expected: Vec<u64> = (0..ids.len() as u64).collect();
    assert_eq!(ids, expected, "got {v}");

    // @step And the added question has the last id and nextQuestionId is one past it
    let questions = v["workUnits"]["AUTH-001"]["questions"]
        .as_array()
        .expect("questions array");
    let last = questions.last().expect("at least one question");
    assert_eq!(
        last["text"].as_str(),
        Some("@human: Anything else?"),
        "got {v}"
    );
    let last_id = last["id"].as_u64().expect("numeric id");
    assert_eq!(
        v["workUnits"]["AUTH-001"]["nextQuestionId"].as_u64(),
        Some(last_id + 1),
        "got {v}"
    );
}

#[test]
fn hotspot_concern_with_several_sentences_is_kept_whole() {
    // @step Given spec/work-units.json contains AUTH-001 with an eventStorm hotspot concern='Should drag-and-drop support multi-select? How to handle edge cases?'
    let tmp = TempDir::new().expect("tempdir");
    let items = json!([hotspot_item(
        0,
        "Drag and drop",
        "Should drag-and-drop support multi-select? How to handle edge cases?",
        false
    )]);
    write_value(tmp.path(), &seed_unit_with_event_storm("AUTH-001", items));

    // @step When I dispatch generate-example-mapping-from-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then a question is appended with text exactly '@human: Should drag-and-drop support multi-select? How to handle edge cases?'
    let v = read_work_units(tmp.path());
    let questions = active_entries(&v, "AUTH-001", "questions");
    assert_eq!(questions.len(), 1, "expected exactly one question");
    assert_eq!(
        questions[0]["text"].as_str(),
        Some("@human: Should drag-and-drop support multi-select? How to handle edge cases?")
    );
}

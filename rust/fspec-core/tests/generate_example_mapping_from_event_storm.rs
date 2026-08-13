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

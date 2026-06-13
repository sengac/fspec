#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/answer-question-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `answer-question` (RPC-196).
// Each #[test] fn maps to one Gherkin scenario; @step comments mirror the Gherkin
// step text verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "answer-question".to_string(),
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

fn seed_unit(id: &str, status: &str) -> Value {
    let mut states_obj = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let arr: Vec<Value> = if *st == status {
            vec![Value::String(id.to_string())]
        } else {
            vec![]
        };
        states_obj.insert((*st).to_string(), Value::Array(arr));
    }
    json!({
        "version": "0.7.1",
        "workUnits": {
            id: {
                "id": id,
                "title": "title",
                "type": "story",
                "status": status,
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": states_obj
    })
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap()
}

fn q(id: u64, text: &str) -> Value {
    json!({
        "id": id,
        "text": text,
        "deleted": false,
        "createdAt": "2026-06-01T00:00:00.000Z"
    })
}

// ---------- scenarios ----------

#[test]
fn add_to_rule_creates_a_proper_rule_item_with_id_from_next_rule_id() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Should we support OAuth?',deleted:false,createdAt:'x'}] and nextRuleId=0
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_unit("AUTH-001", "specifying");
    pre["workUnits"]["AUTH-001"]["questions"] = json!([q(0, "Should we support OAuth?")]);
    pre["workUnits"]["AUTH-001"]["nextRuleId"] = json!(0);
    write_work_units(tmp.path(), &pretty(&pre));

    // @step When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='Yes, Google OAuth' addTo='rule'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0, "answer": "Yes, Google OAuth", "addTo": "rule"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned data contains question='Should we support OAuth?'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["question"].as_str(), Some("Should we support OAuth?"));
    // @step And the returned data contains addedTo='rules'
    assert_eq!(data["addedTo"].as_str(), Some("rules"));
    // @step And the returned data contains addedContent='Yes, Google OAuth'
    assert_eq!(data["addedContent"].as_str(), Some("Yes, Google OAuth"));

    let v = read_work_units(tmp.path());
    // @step And spec/work-units.json on disk shows AUTH-001.questions[0].selected=true
    assert_eq!(
        v["workUnits"]["AUTH-001"]["questions"][0]["selected"].as_bool(),
        Some(true)
    );
    // @step And spec/work-units.json on disk shows AUTH-001.questions[0].answered=true
    assert_eq!(
        v["workUnits"]["AUTH-001"]["questions"][0]["answered"].as_bool(),
        Some(true)
    );
    // @step And spec/work-units.json on disk shows AUTH-001.questions[0].answer='Yes, Google OAuth'
    assert_eq!(
        v["workUnits"]["AUTH-001"]["questions"][0]["answer"].as_str(),
        Some("Yes, Google OAuth")
    );
    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].id=0
    assert_eq!(
        v["workUnits"]["AUTH-001"]["rules"][0]["id"].as_u64(),
        Some(0)
    );
    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].text='Yes, Google OAuth'
    assert_eq!(
        v["workUnits"]["AUTH-001"]["rules"][0]["text"].as_str(),
        Some("Yes, Google OAuth")
    );
    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=false
    assert_eq!(
        v["workUnits"]["AUTH-001"]["rules"][0]["deleted"].as_bool(),
        Some(false)
    );
    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].createdAt is a freshly bumped ISO-8601 timestamp
    let created = v["workUnits"]["AUTH-001"]["rules"][0]["createdAt"]
        .as_str()
        .expect("createdAt");
    assert!(created.len() == 24 && created.ends_with('Z'));
    assert!(!created.starts_with("2026-06-01"));
    // @step And spec/work-units.json on disk shows AUTH-001.nextRuleId=1
    assert_eq!(v["workUnits"]["AUTH-001"]["nextRuleId"].as_u64(), Some(1));
    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = v["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("updatedAt");
    assert!(updated.len() == 24 && updated.ends_with('Z'));
    assert!(!updated.starts_with("2026-06-01"));
}

#[test]
fn add_to_rule_with_preexisting_next_rule_id_increments_sequentially() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}] and nextRuleId=5
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_unit("AUTH-001", "specifying");
    pre["workUnits"]["AUTH-001"]["questions"] = json!([q(0, "Q?")]);
    pre["workUnits"]["AUTH-001"]["nextRuleId"] = json!(5);
    write_work_units(tmp.path(), &pretty(&pre));

    // @step When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='Yes' addTo='rule'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0, "answer": "Yes", "addTo": "rule"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "got {result:?}");

    let v = read_work_units(tmp.path());
    // @step And spec/work-units.json on disk shows AUTH-001.rules[0].id=5
    assert_eq!(
        v["workUnits"]["AUTH-001"]["rules"][0]["id"].as_u64(),
        Some(5)
    );
    // @step And spec/work-units.json on disk shows AUTH-001.nextRuleId=6
    assert_eq!(v["workUnits"]["AUTH-001"]["nextRuleId"].as_u64(), Some(6));
}

#[test]
fn add_to_assumption_appends_raw_string_to_assumptions_array() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_unit("AUTH-001", "specifying");
    pre["workUnits"]["AUTH-001"]["questions"] = json!([q(0, "Q?")]);
    write_work_units(tmp.path(), &pretty(&pre));

    // @step When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='Server is HTTPS only' addTo='assumption'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0, "answer": "Server is HTTPS only", "addTo": "assumption"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "got {result:?}");

    let data: Value = serde_json::from_str(&result.data).expect("parse data");
    // @step And the returned data contains addedTo='assumptions'
    assert_eq!(data["addedTo"].as_str(), Some("assumptions"));

    let v = read_work_units(tmp.path());
    // @step And spec/work-units.json on disk shows AUTH-001.assumptions=['Server is HTTPS only']
    let assumes = v["workUnits"]["AUTH-001"]["assumptions"]
        .as_array()
        .expect("assumptions array");
    assert_eq!(assumes.len(), 1);
    assert_eq!(assumes[0].as_str(), Some("Server is HTTPS only"));
    // @step And spec/work-units.json on disk shows AUTH-001 has no rules added
    assert!(v["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .is_none_or(Vec::is_empty));
}

#[test]
fn add_to_none_with_answer_marks_question_but_does_not_modify_rules_or_assumptions() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_unit("AUTH-001", "specifying");
    pre["workUnits"]["AUTH-001"]["questions"] = json!([q(0, "Q?")]);
    write_work_units(tmp.path(), &pretty(&pre));

    // @step When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='Maybe' addTo='none'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0, "answer": "Maybe", "addTo": "none"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "got {result:?}");

    let data: Value = serde_json::from_str(&result.data).expect("parse data");
    // @step And the returned data does NOT contain addedTo
    assert!(data.get("addedTo").is_none() || data["addedTo"].is_null());
    // @step And the returned data does NOT contain addedContent
    assert!(data.get("addedContent").is_none() || data["addedContent"].is_null());

    let v = read_work_units(tmp.path());
    // @step And spec/work-units.json on disk shows AUTH-001.questions[0].answered=true
    assert_eq!(
        v["workUnits"]["AUTH-001"]["questions"][0]["answered"].as_bool(),
        Some(true)
    );
    // @step And spec/work-units.json on disk shows AUTH-001.questions[0].answer='Maybe'
    assert_eq!(
        v["workUnits"]["AUTH-001"]["questions"][0]["answer"].as_str(),
        Some("Maybe")
    );
    // @step And spec/work-units.json on disk shows AUTH-001 has no rules added
    assert!(v["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .is_none_or(Vec::is_empty));
    // @step And spec/work-units.json on disk shows AUTH-001 has no assumptions added
    assert!(v["workUnits"]["AUTH-001"]["assumptions"]
        .as_array()
        .is_none_or(Vec::is_empty));
}

#[test]
fn no_answer_leaves_answer_untouched_but_still_selects_the_question() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_unit("AUTH-001", "specifying");
    pre["workUnits"]["AUTH-001"]["questions"] = json!([q(0, "Q?")]);
    write_work_units(tmp.path(), &pretty(&pre));

    // @step When I dispatch answer-question with workUnitId='AUTH-001' index=0 and no answer
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "got {result:?}");

    let v = read_work_units(tmp.path());
    // @step And spec/work-units.json on disk shows AUTH-001.questions[0].selected=true
    assert_eq!(
        v["workUnits"]["AUTH-001"]["questions"][0]["selected"].as_bool(),
        Some(true)
    );
    // @step And spec/work-units.json on disk shows AUTH-001.questions[0] has no answered field set
    let q0 = &v["workUnits"]["AUTH-001"]["questions"][0];
    assert!(q0.get("answered").is_none() || q0["answered"].is_null());
    // @step And spec/work-units.json on disk shows AUTH-001.questions[0] has no answer field set
    assert!(q0.get("answer").is_none() || q0["answer"].is_null());
}

#[test]
fn missing_work_unit_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_unit("AUTH-001", "specifying");
    write_work_units(tmp.path(), &pretty(&pre));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch answer-question with workUnitId='NOPE-001' index=0 answer='X' addTo='rule'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "NOPE-001", "index": 0, "answer": "X", "addTo": "rule"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring "Work unit 'NOPE-001' does not exist"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit 'NOPE-001' does not exist"),
        "expected canonical missing message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn non_specifying_status_is_rejected_verbatim() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_unit("AUTH-001", "backlog");
    pre["workUnits"]["AUTH-001"]["questions"] = json!([q(0, "Q?")]);
    write_work_units(tmp.path(), &pretty(&pre));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='X' addTo='rule'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0, "answer": "X", "addTo": "rule"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring "Can only answer questions during discovery/specification phase. AUTH-001 is in 'backlog' state."
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Can only answer questions during discovery/specification phase. AUTH-001 is in 'backlog' state."),
        "expected canonical phase-guard message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn no_questions_array_yields_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with no questions field
    let tmp = TempDir::new().expect("tempdir");
    let pre = seed_unit("AUTH-001", "specifying");
    write_work_units(tmp.path(), &pretty(&pre));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='X' addTo='rule'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0, "answer": "X", "addTo": "rule"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring 'Work unit AUTH-001 has no questions'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit AUTH-001 has no questions"),
        "expected no-questions message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn out_of_range_index_yields_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q1',deleted:false,createdAt:'x'},{id:1,text:'Q2',deleted:false,createdAt:'x'},{id:2,text:'Q3',deleted:false,createdAt:'x'}]
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_unit("AUTH-001", "specifying");
    pre["workUnits"]["AUTH-001"]["questions"] = json!([q(0, "Q1"), q(1, "Q2"), q(2, "Q3")]);
    write_work_units(tmp.path(), &pretty(&pre));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch answer-question with workUnitId='AUTH-001' index=5 answer='X' addTo='rule'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 5, "answer": "X", "addTo": "rule"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring 'Invalid question index 5. Valid range: 0-2'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Invalid question index 5. Valid range: 0-2"),
        "expected out-of-range message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn raw_string_legacy_question_entry_is_rejected() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=['legacy raw string question']
    let tmp = TempDir::new().expect("tempdir");
    let mut pre = seed_unit("AUTH-001", "specifying");
    pre["workUnits"]["AUTH-001"]["questions"] = json!(["legacy raw string question"]);
    write_work_units(tmp.path(), &pretty(&pre));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='X' addTo='rule'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0, "answer": "X", "addTo": "rule"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring 'Question format is invalid. Expected QuestionItem object.'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Question format is invalid. Expected QuestionItem object."),
        "expected legacy-format message; got: {err}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn auto_creates_work_units_then_reports_canonical_missing_source_error() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='X' addTo='rule'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "index": 0, "answer": "X", "addTo": "rule"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success);

    // @step And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit 'AUTH-001' does not exist"),
        "expected canonical missing source; got: {err}"
    );

    // @step And spec/work-units.json now exists on disk with the canonical empty initial structure
    assert!(tmp.path().join("spec/work-units.json").exists());
    let v = read_work_units(tmp.path());
    assert_eq!(v["version"].as_str(), Some("0.7.1"));
    assert!(v["workUnits"].as_object().unwrap().is_empty());
    assert!(v["states"]["backlog"].as_array().unwrap().is_empty());
}

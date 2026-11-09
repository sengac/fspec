//! Feature: spec/features/query-example-mapping-stats-rust-port.feature
//!
//! Dispatcher-level tests for `query-example-mapping-stats` (RPC-260), moved out of
//! rust/fspec/tests/cli_query_example_mapping_stats.rs so each test file maps one feature (COV-075).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn dispatch_qems(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    let req = codelet_fspec_core::DispatchRequest {
        command: "query-example-mapping-stats".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    };
    codelet_fspec_core::dispatch_command(req)
}

/// Build minimal work-units.json containing a single WU with arrays
fn wu_with_arrays(
    id: &str,
    title: &str,
    status: &str,
    rules: usize,
    examples: usize,
    questions: usize,
    assumptions: usize,
) -> String {
    let make_arr = |n: usize, prefix: &str| -> String {
        let items: Vec<String> = (0..n).map(|i| format!(r#""{prefix}-{i}""#)).collect();
        format!("[{}]", items.join(","))
    };
    format!(
        r#"{{
  "version": "0.7.1",
  "workUnits": {{
    "{id}": {{
      "id": "{id}", "title": "{title}", "status": "{status}",
      "createdAt": "x", "updatedAt": "x",
      "rules": {rules_json},
      "examples": {examples_json},
      "questions": {questions_json},
      "assumptions": {assumptions_json}
    }}
  }},
  "states": {{
    "backlog": ["{id}"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }}
}}"#,
        rules_json = make_arr(rules, "rule"),
        examples_json = make_arr(examples, "example"),
        questions_json = make_arr(questions, "question"),
        assumptions_json = make_arr(assumptions, "assumption"),
    )
}

// =========================================================================
// Scenarios from query-example-mapping-stats-rust-port.feature (dispatcher path)
// =========================================================================

#[test]
fn scenario_returns_empty_stats_when_work_units_auto_created_in_empty_workspace() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON has workUnits=[] (empty array)
    assert_eq!(data["workUnits"].as_array().map(Vec::len), Some(0));

    // @step Then the returned JSON has workUnitsWithRules=0, workUnitsWithExamples=0, workUnitsWithQuestions=0, workUnitsWithAssumptions=0
    assert_eq!(data["workUnitsWithRules"].as_u64(), Some(0));
    assert_eq!(data["workUnitsWithExamples"].as_u64(), Some(0));
    assert_eq!(data["workUnitsWithQuestions"].as_u64(), Some(0));
    assert_eq!(data["workUnitsWithAssumptions"].as_u64(), Some(0));

    // @step Then the returned JSON has avgRulesPerWorkUnit=0, avgExamplesPerWorkUnit=0, avgQuestionsPerWorkUnit=0, avgAssumptionsPerWorkUnit=0
    assert_eq!(data["avgRulesPerWorkUnit"].as_f64(), Some(0.0));
    assert_eq!(data["avgExamplesPerWorkUnit"].as_f64(), Some(0.0));
    assert_eq!(data["avgQuestionsPerWorkUnit"].as_f64(), Some(0.0));
    assert_eq!(data["avgAssumptionsPerWorkUnit"].as_f64(), Some(0.0));

    // @step Then spec/work-units.json exists after the call (auto-created by ensure_work_units_file)
    assert!(
        ws.path().join("spec/work-units.json").exists(),
        "spec/work-units.json must be auto-created"
    );
}

#[test]
fn scenario_completeness_score_100_when_rules_and_examples_nonempty_and_no_questions() {
    // @step Given spec/work-units.json contains AUTH-001 with 2 rules, 1 example, 0 questions, 0 assumptions
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &wu_with_arrays("AUTH-001", "Login", "backlog", 2, 1, 0, 0),
    );

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits[0] has workUnitId='AUTH-001'
    assert_eq!(
        data["workUnits"][0]["workUnitId"].as_str(),
        Some("AUTH-001")
    );

    // @step Then the returned JSON workUnits[0] has rules=2, examples=1, questions=0, assumptions=0
    assert_eq!(data["workUnits"][0]["rules"].as_u64(), Some(2));
    assert_eq!(data["workUnits"][0]["examples"].as_u64(), Some(1));
    assert_eq!(data["workUnits"][0]["questions"].as_u64(), Some(0));
    assert_eq!(data["workUnits"][0]["assumptions"].as_u64(), Some(0));

    // @step Then the returned JSON workUnits[0] has completenessScore=100
    assert_eq!(
        data["workUnits"][0]["completenessScore"].as_u64(),
        Some(100)
    );
}

#[test]
fn scenario_completeness_score_0_when_only_questions() {
    // @step Given spec/work-units.json contains AUTH-002 with 0 rules, 0 examples, 1 question, 0 assumptions
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &wu_with_arrays("AUTH-002", "X", "backlog", 0, 0, 1, 0),
    );

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits[0] has completenessScore=0
    assert_eq!(data["workUnits"][0]["completenessScore"].as_u64(), Some(0));
}

#[test]
fn scenario_completeness_score_66_with_rules_only_and_no_questions() {
    // @step Given spec/work-units.json contains AUTH-001 with 1 rule, 0 examples, 0 questions, 0 assumptions
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &wu_with_arrays("AUTH-001", "X", "backlog", 1, 0, 0, 0),
    );

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits[0] has completenessScore=66
    assert_eq!(data["workUnits"][0]["completenessScore"].as_u64(), Some(66));
}

#[test]
fn scenario_completeness_score_67_with_examples_only_and_no_questions() {
    // @step Given spec/work-units.json contains AUTH-001 with 0 rules, 1 example, 0 questions, 0 assumptions
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &wu_with_arrays("AUTH-001", "X", "backlog", 0, 1, 0, 0),
    );

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits[0] has completenessScore=67
    assert_eq!(data["workUnits"][0]["completenessScore"].as_u64(), Some(67));
}

#[test]
fn scenario_aggregate_counts_and_averages_reflect_every_retained_work_unit() {
    // @step Given spec/work-units.json contains AUTH-001 (2 rules, 1 example, 0 questions, 0 assumptions) and AUTH-002 (0 rules, 0 examples, 1 question, 0 assumptions)
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x","rules":["r0","r1"],"examples":["e0"],"questions":[],"assumptions":[]},
    "AUTH-002": {"id":"AUTH-002","title":"B","status":"backlog","createdAt":"x","updatedAt":"x","rules":[],"examples":[],"questions":["q0"],"assumptions":[]}
  },
  "states": {"backlog":["AUTH-001","AUTH-002"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);
    assert!(result.success, "must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON has workUnitsWithRules=1, workUnitsWithExamples=1, workUnitsWithQuestions=1, workUnitsWithAssumptions=0
    assert_eq!(data["workUnitsWithRules"].as_u64(), Some(1));
    assert_eq!(data["workUnitsWithExamples"].as_u64(), Some(1));
    assert_eq!(data["workUnitsWithQuestions"].as_u64(), Some(1));
    assert_eq!(data["workUnitsWithAssumptions"].as_u64(), Some(0));

    // @step Then the returned JSON has avgRulesPerWorkUnit=1
    assert_eq!(data["avgRulesPerWorkUnit"].as_f64(), Some(1.0));

    // @step Then the returned JSON has avgExamplesPerWorkUnit=0.5
    assert_eq!(data["avgExamplesPerWorkUnit"].as_f64(), Some(0.5));

    // @step Then the returned JSON has avgQuestionsPerWorkUnit=0.5
    assert_eq!(data["avgQuestionsPerWorkUnit"].as_f64(), Some(0.5));
}

#[test]
fn scenario_work_unit_id_filter_narrows_to_single_work_unit() {
    // @step Given spec/work-units.json contains AUTH-001 (2 rules, 1 example), AUTH-002 (0 rules, 1 question), AUTH-003 (1 rule)
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x","rules":["r0","r1"],"examples":["e0"]},
    "AUTH-002": {"id":"AUTH-002","title":"B","status":"backlog","createdAt":"x","updatedAt":"x","questions":["q0"]},
    "AUTH-003": {"id":"AUTH-003","title":"C","status":"backlog","createdAt":"x","updatedAt":"x","rules":["r0"]}
  },
  "states": {"backlog":["AUTH-001","AUTH-002","AUTH-003"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with workUnitId='AUTH-001' and format='json'
    let result = dispatch_qems(ws.path(), r#"{"workUnitId":"AUTH-001","format":"json"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits has exactly one entry whose workUnitId='AUTH-001'
    assert_eq!(data["workUnits"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        data["workUnits"][0]["workUnitId"].as_str(),
        Some("AUTH-001")
    );

    // @step Then the returned JSON has workUnitsWithRules=1 and workUnitsWithQuestions=0
    assert_eq!(data["workUnitsWithRules"].as_u64(), Some(1));
    assert_eq!(data["workUnitsWithQuestions"].as_u64(), Some(0));

    // @step Then the returned JSON has avgRulesPerWorkUnit=2 and avgQuestionsPerWorkUnit=0
    assert_eq!(data["avgRulesPerWorkUnit"].as_f64(), Some(2.0));
    assert_eq!(data["avgQuestionsPerWorkUnit"].as_f64(), Some(0.0));
}

#[test]
fn scenario_work_unit_id_filter_against_missing_id_surfaces_error() {
    // @step Given spec/work-units.json contains AUTH-001, AUTH-002, AUTH-003
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x"},
    "AUTH-002": {"id":"AUTH-002","title":"B","status":"backlog","createdAt":"x","updatedAt":"x"},
    "AUTH-003": {"id":"AUTH-003","title":"C","status":"backlog","createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":["AUTH-001","AUTH-002","AUTH-003"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with workUnitId='NOPE-999' and format='json'
    let result = dispatch_qems(ws.path(), r#"{"workUnitId":"NOPE-999","format":"json"}"#);

    // @step Then the dispatcher returns success=false with an error message containing the substring "Work unit 'NOPE-999' does not exist"
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_ref()
            .map(|e| e.contains("Work unit 'NOPE-999' does not exist"))
            .unwrap_or(false),
        "error must mention missing work unit; got {result:?}"
    );
}

#[test]
fn scenario_has_questions_true_keeps_only_units_with_questions() {
    // @step Given spec/work-units.json contains AUTH-001 with 1 question and AUTH-002 with 0 questions
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x","questions":["q0"]},
    "AUTH-002": {"id":"AUTH-002","title":"B","status":"backlog","createdAt":"x","updatedAt":"x","questions":[]}
  },
  "states": {"backlog":["AUTH-001","AUTH-002"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with hasQuestions=true and format='json'
    let result = dispatch_qems(ws.path(), r#"{"hasQuestions":true,"format":"json"}"#);
    assert!(result.success, "must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits has exactly one entry whose workUnitId='AUTH-001'
    assert_eq!(data["workUnits"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        data["workUnits"][0]["workUnitId"].as_str(),
        Some("AUTH-001")
    );
}

#[test]
fn scenario_has_questions_false_keeps_only_units_with_zero_questions() {
    // @step Given spec/work-units.json contains AUTH-001 with 1 question and AUTH-002 with 0 questions
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x","questions":["q0"]},
    "AUTH-002": {"id":"AUTH-002","title":"B","status":"backlog","createdAt":"x","updatedAt":"x","questions":[]}
  },
  "states": {"backlog":["AUTH-001","AUTH-002"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with hasQuestions=false and format='json'
    let result = dispatch_qems(ws.path(), r#"{"hasQuestions":false,"format":"json"}"#);
    assert!(result.success, "must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits has exactly one entry whose workUnitId='AUTH-002'
    assert_eq!(data["workUnits"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        data["workUnits"][0]["workUnitId"].as_str(),
        Some("AUTH-002")
    );
}

#[test]
fn scenario_questions_for_alice_keeps_only_units_mentioning_alice() {
    // @step Given spec/work-units.json contains AUTH-001 with question '@alice should we cache?' and AUTH-002 with question '@bob review'
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"A","status":"backlog","createdAt":"x","updatedAt":"x","questions":["@alice should we cache?"]},
    "AUTH-002": {"id":"AUTH-002","title":"B","status":"backlog","createdAt":"x","updatedAt":"x","questions":["@bob review"]}
  },
  "states": {"backlog":["AUTH-001","AUTH-002"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with questionsFor='alice' and format='json'
    let result = dispatch_qems(ws.path(), r#"{"questionsFor":"alice","format":"json"}"#);
    assert!(result.success, "must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits has exactly one entry whose workUnitId='AUTH-001'
    assert_eq!(data["workUnits"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        data["workUnits"][0]["workUnitId"].as_str(),
        Some("AUTH-001")
    );
}

#[test]
fn scenario_result_json_field_order_matches_ts_shape() {
    // @step Given spec/work-units.json contains AUTH-001 with 1 rule and 1 example
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &wu_with_arrays("AUTH-001", "A", "backlog", 1, 1, 0, 0),
    );

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the returned JSON field declaration order is workUnits, workUnitsWithRules, workUnitsWithExamples, workUnitsWithQuestions, workUnitsWithAssumptions, avgRulesPerWorkUnit, avgExamplesPerWorkUnit, avgQuestionsPerWorkUnit, avgAssumptionsPerWorkUnit
    let expected_order = [
        "workUnits",
        "workUnitsWithRules",
        "workUnitsWithExamples",
        "workUnitsWithQuestions",
        "workUnitsWithAssumptions",
        "avgRulesPerWorkUnit",
        "avgExamplesPerWorkUnit",
        "avgQuestionsPerWorkUnit",
        "avgAssumptionsPerWorkUnit",
    ];
    let mut last_pos: i64 = -1;
    for field in expected_order {
        let key_pattern = format!("\"{field}\"");
        let pos = result
            .data
            .find(&key_pattern)
            .unwrap_or_else(|| panic!("field {field} missing from data:\n{}", result.data))
            as i64;
        assert!(
            pos > last_pos,
            "field order broken at {field}: pos {pos} <= last {last_pos}\nin:\n{}",
            result.data
        );
        last_pos = pos;
    }
}

#[test]
fn scenario_escalates_malformed_work_units_json_dispatcher() {
    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), "{ not json");

    // @step When I dispatch query-example-mapping-stats against that project root
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_ref()
            .map(|e| e.contains("Failed to parse work-units.json"))
            .unwrap_or(false),
        "error must mention parse failure; got {result:?}"
    );
}

#[test]
fn scenario_per_work_unit_stats_record_carries_title_and_status() {
    // @step Given spec/work-units.json contains AUTH-001 with title 'Login flow' and status 'implementing'
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {"id":"AUTH-001","title":"Login flow","status":"implementing","createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":[],"specifying":[],"testing":[],"implementing":["AUTH-001"],"validating":[],"done":[],"blocked":[]}
}"#;
    write_work_units(ws.path(), raw);

    // @step When I dispatch query-example-mapping-stats with format='json'
    let result = dispatch_qems(ws.path(), r#"{"format":"json"}"#);
    assert!(result.success, "must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");

    // @step Then the returned JSON workUnits[0] has title='Login flow' and status='implementing'
    assert_eq!(data["workUnits"][0]["title"].as_str(), Some("Login flow"));
    assert_eq!(
        data["workUnits"][0]["status"].as_str(),
        Some("implementing")
    );
}

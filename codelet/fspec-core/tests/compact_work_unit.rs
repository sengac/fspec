#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/compact-work-unit-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `compact-work-unit`
// (RPC-206). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "compact-work-unit".to_string(),
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
    serde_json::from_str(&raw).expect("valid JSON")
}

/// Build a rules array string: `count` total rules where the first
/// `deleted_count` are soft-deleted. ids run 0..count.
fn rules_json(count: usize, deleted_count: usize) -> String {
    let mut out = Vec::new();
    for i in 0..count {
        let deleted = i < deleted_count;
        out.push(format!(
            r#"{{"id":{i},"text":"rule {i}","deleted":{deleted},"createdAt":"x"}}"#
        ));
    }
    format!("[{}]", out.join(","))
}

/// Build a work-units.json document with a single AUTH-001 unit carrying the
/// supplied status, rules array, nextRuleId, and optional meta block.
fn doc(status: &str, rules: &str, next_rule_id: usize, with_meta: bool) -> String {
    let meta = if with_meta {
        r#""meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },"#
    } else {
        ""
    };
    format!(
        r#"{{
  "version": "0.7.1",
  {meta}
  "workUnits": {{
    "AUTH-001": {{
      "id": "AUTH-001", "title": "Login", "status": "{status}",
      "rules": {rules}, "nextRuleId": {next_rule_id},
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
    }}
  }},
  "states": {{
    "backlog": [], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
  }}
}}"#
    )
}

/// Count live (non-deleted) rules on AUTH-001.
fn rules_array(data: &Value) -> Vec<Value> {
    data["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

// ---------- scenarios ----------

#[test]
fn dispatcher_removes_deleted_rules_and_renumbers_survivors() {
    // Scenario: Dispatcher removes soft-deleted rules and renumbers the survivors

    // @step Given spec/work-units.json contains work unit AUTH-001 with status='done' having 3 deleted rules and 7 live rules
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &doc("done", &rules_json(10, 3), 10, true));

    // @step When I dispatch compact-work-unit with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the AUTH-001 rules array in spec/work-units.json contains 7 items
    let data = read_work_units(tmp.path());
    let rules = rules_array(&data);
    assert_eq!(rules.len(), 7, "expected 7 surviving rules; got {data}");

    // @step And the surviving AUTH-001 rules have sequential ids 0 through 6
    let ids: Vec<i64> = rules.iter().filter_map(|r| r["id"].as_i64()).collect();
    assert_eq!(ids, (0..7).collect::<Vec<i64>>(), "ids must be renumbered 0..6; got {ids:?}");

    // @step And nextRuleId on AUTH-001 equals 7
    assert_eq!(
        data["workUnits"]["AUTH-001"]["nextRuleId"].as_i64(),
        Some(7),
        "nextRuleId must reset to 7; got {data}"
    );
}

#[test]
fn dispatcher_rejects_compaction_of_missing_work_unit() {
    // Scenario: Dispatcher rejects compaction of a missing work unit

    // @step Given spec/work-units.json contains work unit AUTH-001 with status='done'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &doc("done", &rules_json(0, 0), 0, true));

    // @step When I dispatch compact-work-unit with workUnitId='MISSING-999'
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "MISSING-999"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Work unit 'MISSING-999' does not exist"
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Work unit 'MISSING-999' does not exist"),
        "missing canonical not-found text: {msg}"
    );
}

#[test]
fn dispatcher_requires_force_when_status_not_done() {
    // Scenario: Dispatcher requires force when status is not done

    // @step Given spec/work-units.json contains work unit AUTH-001 with status='specifying' having 1 deleted rule
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &doc("specifying", &rules_json(1, 1), 1, true));

    // @step When I dispatch compact-work-unit with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring "Cannot compact work unit in 'specifying' status. Use --force to confirm compaction during active development."
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Cannot compact work unit in 'specifying' status. Use --force to confirm compaction during active development."),
        "missing canonical force-gate text: {msg}"
    );

    // @step And the AUTH-001 rules array in spec/work-units.json still contains the deleted rule
    let data = read_work_units(tmp.path());
    assert_eq!(rules_array(&data).len(), 1, "deleted rule must be preserved; got {data}");
}

#[test]
fn dispatcher_compacts_during_non_done_status_with_force() {
    // Scenario: Dispatcher compacts during non-done status when force is set

    // @step Given spec/work-units.json contains work unit AUTH-001 with status='specifying' having 1 deleted rule and 2 live rules
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &doc("specifying", &rules_json(3, 1), 3, true));

    // @step When I dispatch compact-work-unit with workUnitId='AUTH-001' and force=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "force": true}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the AUTH-001 rules array in spec/work-units.json contains 2 items
    let data = read_work_units(tmp.path());
    assert_eq!(rules_array(&data).len(), 2, "expected 2 surviving rules; got {data}");
}

#[test]
fn dispatcher_resets_counters_when_no_deleted_items() {
    // Scenario: Dispatcher resets counters when there are no deleted items

    // @step Given spec/work-units.json contains work unit AUTH-001 with status='done' having 3 live rules and no deleted items
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &doc("done", &rules_json(3, 0), 99, true));

    // @step When I dispatch compact-work-unit with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the AUTH-001 rules array in spec/work-units.json contains 3 items
    let data = read_work_units(tmp.path());
    assert_eq!(rules_array(&data).len(), 3, "expected 3 rules; got {data}");

    // @step And nextRuleId on AUTH-001 equals 3
    assert_eq!(
        data["workUnits"]["AUTH-001"]["nextRuleId"].as_i64(),
        Some(3),
        "nextRuleId must reset to 3; got {data}"
    );
}

#[test]
fn dispatcher_updates_work_unit_and_meta_timestamps() {
    // Scenario: Dispatcher updates the work unit and meta timestamps

    // @step Given spec/work-units.json contains work unit AUTH-001 with status='done' having 1 deleted rule and a meta.lastUpdated value
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &doc("done", &rules_json(1, 1), 1, true));

    // @step When I dispatch compact-work-unit with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the AUTH-001 updatedAt field in spec/work-units.json is a non-empty ISO-8601 timestamp
    let data = read_work_units(tmp.path());
    let updated = data["workUnits"]["AUTH-001"]["updatedAt"].as_str().unwrap_or("");
    assert!(
        updated.contains('T') && updated.ends_with('Z') && !updated.is_empty(),
        "updatedAt must be a non-empty ISO-8601 timestamp; got '{updated}'"
    );

    // @step And the meta.lastUpdated field in spec/work-units.json is a non-empty ISO-8601 timestamp
    let last = data["meta"]["lastUpdated"].as_str().unwrap_or("");
    assert!(
        last.contains('T') && last.ends_with('Z') && !last.is_empty(),
        "meta.lastUpdated must be a non-empty ISO-8601 timestamp; got '{last}'"
    );
}

#[test]
fn dispatcher_fails_fast_when_required_args_are_missing() {
    // Scenario: Dispatcher fails fast when required args are missing

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch compact-work-unit with no workUnitId field in the args
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected missing-field error: {result:?}");

    // @step And the error message contains the substring 'Invalid args for fspec command compact-work-unit'
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Invalid args for fspec command compact-work-unit"),
        "missing canonical InvalidArgs prefix: {msg}"
    );
}

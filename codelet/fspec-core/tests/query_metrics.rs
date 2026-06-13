#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/query-metrics-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `query-metrics`
// (RPC-261). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "query-metrics".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

/// ISO-8601 stamp `2026-01-01T00:00:00.000Z` + `hours` hours.
fn iso_hour(hours: u32) -> String {
    let h = hours % 24;
    let extra_days = hours / 24;
    // Stay within January 2026 for the cases we use (<= 31 days).
    let day = 1 + extra_days;
    format!("2026-01-{day:02}T{h:02}:00:00.000Z")
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Aggregate JSON with no filter computes totals, completion and averages
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn aggregate_json_with_no_filter_computes_totals_completion_and_averages() {
    // @step Given spec/work-units.json contains AUTH-001 (status done, stateHistory backlog→done spanning 4 hours), AUTH-002 (backlog with stateHistory), and AUTH-003 (backlog, no stateHistory)
    let tmp = TempDir::new().expect("tempdir");
    let auth1_t0 = iso_hour(0);
    let auth1_t4 = iso_hour(4);
    let auth2_t0 = iso_hour(0);
    let auth2_t1 = iso_hour(1);
    let raw = format!(
        r#"{{
  "workUnits": {{
    "AUTH-001": {{
      "id": "AUTH-001",
      "title": "First",
      "type": "story",
      "status": "done",
      "createdAt": "{auth1_t0}",
      "updatedAt": "{auth1_t4}",
      "stateHistory": [
        {{ "state": "backlog", "timestamp": "{auth1_t0}" }},
        {{ "state": "done",    "timestamp": "{auth1_t4}" }}
      ]
    }},
    "AUTH-002": {{
      "id": "AUTH-002",
      "title": "Second",
      "type": "story",
      "status": "backlog",
      "createdAt": "{auth2_t0}",
      "updatedAt": "{auth2_t1}",
      "stateHistory": [
        {{ "state": "backlog", "timestamp": "{auth2_t0}" }},
        {{ "state": "specifying", "timestamp": "{auth2_t1}" }}
      ]
    }},
    "AUTH-003": {{
      "id": "AUTH-003",
      "title": "Third",
      "type": "story",
      "status": "backlog",
      "createdAt": "{auth2_t0}",
      "updatedAt": "{auth2_t0}"
    }}
  }},
  "states": {{
    "backlog": ["AUTH-002", "AUTH-003"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
  }}
}}"#
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch query-metrics with format='json' and no other args
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);

    // @step Then DispatchResult.data parses as JSON whose aggregateMetrics.totalWorkUnits equals 3
    assert_eq!(data["aggregateMetrics"]["totalWorkUnits"].as_u64(), Some(3));

    // @step Then aggregateMetrics.completedWorkUnits equals 1
    assert_eq!(
        data["aggregateMetrics"]["completedWorkUnits"].as_u64(),
        Some(1)
    );

    // @step Then aggregateMetrics.averageCycleTime equals '4 hours'
    assert_eq!(
        data["aggregateMetrics"]["averageCycleTime"].as_str(),
        Some("4 hours")
    );

    // @step Then aggregateMetrics.byType has keys story, task, bug in that exact order with story.count=3, task.count=0, bug.count=0
    let by_type = data["aggregateMetrics"]["byType"]
        .as_object()
        .expect("byType is an object");
    // Verify by-type entry counts via direct key access (Value::Object lookup
    // is order-independent).
    assert_eq!(by_type["story"]["count"].as_u64(), Some(3));
    assert_eq!(by_type["task"]["count"].as_u64(), Some(0));
    assert_eq!(by_type["bug"]["count"].as_u64(), Some(0));
    // Verify byType key ORDER by byte-position scan of the raw JSON
    // payload — mirrors the tag_stats pattern at
    // codelet/fspec-core/tests/tag_stats.rs:421. serde_json::Map only
    // preserves insertion order when the `preserve_order` cargo feature
    // is active; that feature is not enabled in the fspec-core-only test
    // build, but `to_string_pretty` always emits keys in IndexMap order
    // for our Serialize impl. Scanning the raw string sidesteps the
    // re-parse alphabetisation.
    let raw = &result.data;
    let story = raw.find("\"story\"").expect("story key");
    let task = raw.find("\"task\"").expect("task key");
    let bug = raw.find("\"bug\"").expect("bug key");
    assert!(
        story < task && task < bug,
        "byType key order must be story,task,bug; got story={story} task={task} bug={bug}\n{raw}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Single work unit JSON returns cycleTime and timePerState
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn single_work_unit_json_returns_cycletime_and_timeperstate() {
    // @step Given spec/work-units.json contains AUTH-001 with stateHistory entries at hour 0 (backlog), hour 2 (specifying), hour 5 (done)
    let tmp = TempDir::new().expect("tempdir");
    let t0 = iso_hour(0);
    let t2 = iso_hour(2);
    let t5 = iso_hour(5);
    let raw = format!(
        r#"{{
  "workUnits": {{
    "AUTH-001": {{
      "id": "AUTH-001",
      "title": "Login",
      "type": "story",
      "status": "done",
      "createdAt": "{t0}",
      "updatedAt": "{t5}",
      "stateHistory": [
        {{ "state": "backlog",    "timestamp": "{t0}" }},
        {{ "state": "specifying", "timestamp": "{t2}" }},
        {{ "state": "done",       "timestamp": "{t5}" }}
      ]
    }}
  }},
  "states": {{
    "backlog": [], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
  }}
}}"#
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch query-metrics with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);

    // @step Then DispatchResult.data parses as JSON with cycleTime='5 hours'
    assert_eq!(data["cycleTime"].as_str(), Some("5 hours"));

    // @step Then timePerState.backlog='2 hours' and timePerState.specifying='3 hours'
    assert_eq!(data["timePerState"]["backlog"].as_str(), Some("2 hours"));
    assert_eq!(data["timePerState"]["specifying"].as_str(), Some("3 hours"));

    // @step Then the JSON does NOT contain an aggregateMetrics key
    assert!(
        data.get("aggregateMetrics").is_none(),
        "must not have aggregateMetrics key; got {data}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Unknown work unit id fails with wrapped error
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn unknown_work_unit_id_fails_with_wrapped_error() {
    // @step Given spec/work-units.json contains AUTH-001 but no NOPE-999
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        r#"{
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "t", "type": "story", "status": "backlog",
                  "createdAt": "x", "updatedAt": "x" }
  },
  "states": { "backlog": ["AUTH-001"], "specifying": [], "testing": [],
              "implementing": [], "validating": [], "done": [], "blocked": [] }
}"#,
    );

    // @step When I dispatch query-metrics with workUnitId='NOPE-999'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "NOPE-999" })));

    // @step Then the dispatcher returns success=false with an error message containing 'Failed to query metrics: Work unit NOPE-999 not found'
    assert!(!result.success, "expected success=false, got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Failed to query metrics: Work unit NOPE-999 not found"),
        "unexpected error: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Work unit without state history fails with wrapped error
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn work_unit_without_state_history_fails_with_wrapped_error() {
    // @step Given spec/work-units.json contains AUTH-001 with no stateHistory field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        r#"{
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "t", "type": "story", "status": "backlog",
                  "createdAt": "x", "updatedAt": "x" }
  },
  "states": { "backlog": ["AUTH-001"], "specifying": [], "testing": [],
              "implementing": [], "validating": [], "done": [], "blocked": [] }
}"#,
    );

    // @step When I dispatch query-metrics with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=false with an error message containing 'Failed to query metrics: Work unit AUTH-001 has no state history'
    assert!(!result.success, "expected success=false, got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Failed to query metrics: Work unit AUTH-001 has no state history"),
        "unexpected error: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Type filter omits byType from the result
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn type_filter_omits_bytype_from_the_result() {
    // @step Given spec/work-units.json contains a story AUTH-001, a task TASK-001 and a bug BUG-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        r#"{
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "t", "type": "story", "status": "backlog",
                  "createdAt": "x", "updatedAt": "x" },
    "TASK-001": { "id": "TASK-001", "title": "t", "type": "task", "status": "backlog",
                  "createdAt": "x", "updatedAt": "x" },
    "BUG-001":  { "id": "BUG-001",  "title": "t", "type": "bug",  "status": "backlog",
                  "createdAt": "x", "updatedAt": "x" }
  },
  "states": { "backlog": ["AUTH-001","TASK-001","BUG-001"], "specifying": [], "testing": [],
              "implementing": [], "validating": [], "done": [], "blocked": [] }
}"#,
    );

    // @step When I dispatch query-metrics with type='bug' and format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "type": "bug", "format": "json" })));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);

    // @step Then DispatchResult.data.aggregateMetrics.totalWorkUnits equals 1
    assert_eq!(data["aggregateMetrics"]["totalWorkUnits"].as_u64(), Some(1));

    // @step Then DispatchResult.data.aggregateMetrics does NOT contain a byType key
    let agg = data["aggregateMetrics"]
        .as_object()
        .expect("aggregateMetrics object");
    assert!(
        !agg.contains_key("byType"),
        "type filter must omit byType; got {agg:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Empty work units map preserves the three byType keys with zero counts
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn empty_work_units_map_preserves_three_bytype_keys_with_zero_counts() {
    // @step Given spec/work-units.json exists with an empty workUnits object
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        r#"{
  "workUnits": {},
  "states": { "backlog": [], "specifying": [], "testing": [],
              "implementing": [], "validating": [], "done": [], "blocked": [] }
}"#,
    );

    // @step When I dispatch query-metrics with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);

    // @step Then aggregateMetrics.totalWorkUnits=0 and aggregateMetrics.completedWorkUnits=0
    assert_eq!(data["aggregateMetrics"]["totalWorkUnits"].as_u64(), Some(0));
    assert_eq!(
        data["aggregateMetrics"]["completedWorkUnits"].as_u64(),
        Some(0)
    );

    // @step Then aggregateMetrics does NOT contain an averageCycleTime key
    let agg = data["aggregateMetrics"]
        .as_object()
        .expect("aggregateMetrics object");
    assert!(
        !agg.contains_key("averageCycleTime"),
        "must omit averageCycleTime when no completed-with-history units; got {agg:?}"
    );

    // @step Then aggregateMetrics.byType keys are exactly story, task, bug in that order with all counts equal to 0 and no averageCycleTime keys
    let by_type = data["aggregateMetrics"]["byType"]
        .as_object()
        .expect("byType object");
    for key in &["story", "task", "bug"] {
        assert_eq!(by_type[*key]["count"].as_u64(), Some(0), "{key} count");
        let entry = by_type[*key].as_object().expect("entry object");
        assert!(
            !entry.contains_key("averageCycleTime"),
            "{key} must omit averageCycleTime when count==0; got {entry:?}"
        );
    }
    // Verify byType key order by byte-position scan of the raw JSON
    // payload (see notes in the first scenario above).
    let raw = &result.data;
    let story = raw.find("\"story\"").expect("story key");
    let task = raw.find("\"task\"").expect("task key");
    let bug = raw.find("\"bug\"").expect("bug key");
    assert!(
        story < task && task < bug,
        "byType key order must be story,task,bug; got story={story} task={task} bug={bug}\n{raw}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Missing work-units.json escalates as a wrapped Failed to query metrics error
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn missing_work_units_json_escalates_as_wrapped_failed_to_query_metrics_error() {
    // @step Given the project root has no spec/work-units.json
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch query-metrics with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=false with an error message starting with 'Failed to query metrics:'
    assert!(!result.success, "expected success=false, got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Failed to query metrics:"),
        "unexpected error: {err}"
    );

    // @step Then spec/work-units.json still does not exist after the call
    assert!(
        !tmp.path().join("spec/work-units.json").exists(),
        "must NOT auto-create spec/work-units.json"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Text aggregate output is human-readable and not JSON
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn text_aggregate_output_is_human_readable_and_not_json() {
    // @step Given spec/work-units.json contains AUTH-001 (status done, stateHistory 0h→2h) and AUTH-002 (backlog)
    let tmp = TempDir::new().expect("tempdir");
    let t0 = iso_hour(0);
    let t2 = iso_hour(2);
    let raw = format!(
        r#"{{
  "workUnits": {{
    "AUTH-001": {{
      "id": "AUTH-001", "title": "t", "type": "story", "status": "done",
      "createdAt": "{t0}", "updatedAt": "{t2}",
      "stateHistory": [
        {{ "state": "backlog", "timestamp": "{t0}" }},
        {{ "state": "done",    "timestamp": "{t2}" }}
      ]
    }},
    "AUTH-002": {{
      "id": "AUTH-002", "title": "t", "type": "story", "status": "backlog",
      "createdAt": "{t0}", "updatedAt": "{t0}"
    }}
  }},
  "states": {{
    "backlog": ["AUTH-002"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
  }}
}}"#
    );
    write_work_units(tmp.path(), &raw);

    // @step When I dispatch query-metrics with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));
    assert!(result.success, "expected success=true, got {result:?}");
    let out = result.data;

    // @step Then DispatchResult.data contains the substring 'Project Metrics'
    assert!(out.contains("Project Metrics"), "got:\n{out}");

    // @step Then DispatchResult.data contains the exact line 'Total Work Units: 2'
    assert!(
        out.lines().any(|l| l == "Total Work Units: 2"),
        "want 'Total Work Units: 2' line; got:\n{out}"
    );

    // @step Then DispatchResult.data contains the exact line 'Completed Work Units: 1'
    assert!(
        out.lines().any(|l| l == "Completed Work Units: 1"),
        "want 'Completed Work Units: 1' line; got:\n{out}"
    );

    // @step Then DispatchResult.data contains the substring 'By Type:'
    assert!(out.contains("By Type:"), "got:\n{out}");

    // @step Then DispatchResult.data does NOT start with '{'
    assert!(
        !out.trim_start().starts_with('{'),
        "text output must not be JSON; got:\n{out}"
    );
}

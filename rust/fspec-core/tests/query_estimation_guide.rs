//! Feature: spec/features/query-estimation-guide-rust-port.feature
//!
//! Dispatcher-level tests for `query-estimation-guide` (RPC-259), moved out of
//! rust/fspec/tests/cli_query_estimation_guide.rs so each test file maps one feature (COV-075).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "query-estimation-guide".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

/// Build a work-units.json. `entries` is a slice of (id, status, extra_fields).
fn work_units_with(entries: &[(&str, &str, Value)]) -> String {
    let mut wus = serde_json::Map::new();
    for (id, status, extra) in entries {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("status".into(), Value::String((*status).to_string()));
        obj.insert("createdAt".into(), Value::String("x".to_string()));
        obj.insert("updatedAt".into(), Value::String("x".to_string()));
        if let Value::Object(map) = extra {
            for (k, v) in map {
                obj.insert(k.clone(), v.clone());
            }
        }
        wus.insert((*id).to_string(), Value::Object(obj));
    }
    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": {
            "backlog": [], "specifying": [], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    }))
    .unwrap()
}

// ─────────────────────────────────────────────────────────────────────────
// rust-port scenarios (dispatcher contract)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_empty_workspace_returns_empty_patterns() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the returned JSON has patterns=[]
    let data = parse_data(&result.data);
    assert_eq!(data["patterns"].as_array().map(Vec::len), Some(0));

    // @step And spec/work-units.json exists after the call (auto-created by ensure_work_units_file)
    assert!(tmp.path().join("spec/work-units.json").exists());
}

#[test]
fn scenario_dispatcher_ignores_non_done_units() {
    // @step Given spec/work-units.json contains A with status='backlog', estimate=3, iterations=1
    // @step And spec/work-units.json also contains B with status='implementing', estimate=5, iterations=2
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("A", "backlog", json!({ "estimate": 3, "iterations": 1 })),
            (
                "B",
                "implementing",
                json!({ "estimate": 5, "iterations": 2 }),
            ),
        ]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has patterns=[]
    assert_eq!(data["patterns"].as_array().map(Vec::len), Some(0));
}

#[test]
fn scenario_dispatcher_skips_done_unit_missing_iterations() {
    // @step Given spec/work-units.json contains A with status='done', estimate=3 and no iterations field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "done", json!({ "estimate": 3 }))]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has patterns=[]
    assert_eq!(data["patterns"].as_array().map(Vec::len), Some(0));
}

#[test]
fn scenario_dispatcher_skips_done_unit_missing_estimate() {
    // @step Given spec/work-units.json contains A with status='done', iterations=1 and no estimate field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "done", json!({ "iterations": 1 }))]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has patterns=[]
    assert_eq!(data["patterns"].as_array().map(Vec::len), Some(0));
}

#[test]
fn scenario_dispatcher_single_done_unit_yields_low_confidence() {
    // @step Given spec/work-units.json contains A with status='done', estimate=3, iterations=1
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "done", json!({ "estimate": 3, "iterations": 1 }))]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has patterns containing exactly one entry {points=3, expectedIterations='1-1', confidence='low'}
    let arr = data["patterns"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["points"].as_u64(), Some(3));
    assert_eq!(arr[0]["expectedIterations"].as_str(), Some("1-1"));
    assert_eq!(arr[0]["confidence"].as_str(), Some("low"));
}

#[test]
fn scenario_dispatcher_two_done_units_yield_medium_confidence() {
    // @step Given spec/work-units.json contains A with status='done', estimate=3, iterations=1 and B with status='done', estimate=3, iterations=2
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("A", "done", json!({ "estimate": 3, "iterations": 1 })),
            ("B", "done", json!({ "estimate": 3, "iterations": 2 })),
        ]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has patterns containing exactly one entry {points=3, expectedIterations='1-2', confidence='medium'}
    let arr = data["patterns"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["points"].as_u64(), Some(3));
    assert_eq!(arr[0]["expectedIterations"].as_str(), Some("1-2"));
    assert_eq!(arr[0]["confidence"].as_str(), Some("medium"));
}

#[test]
fn scenario_dispatcher_four_done_units_yield_high_confidence() {
    // @step Given spec/work-units.json contains four done units all with estimate=5 and iterations 1, 2, 3, 4 respectively
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("A", "done", json!({ "estimate": 5, "iterations": 1 })),
            ("B", "done", json!({ "estimate": 5, "iterations": 2 })),
            ("C", "done", json!({ "estimate": 5, "iterations": 3 })),
            ("D", "done", json!({ "estimate": 5, "iterations": 4 })),
        ]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has patterns containing exactly one entry {points=5, expectedIterations='1-4', confidence='high'}
    let arr = data["patterns"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["points"].as_u64(), Some(5));
    assert_eq!(arr[0]["expectedIterations"].as_str(), Some("1-4"));
    assert_eq!(arr[0]["confidence"].as_str(), Some("high"));
}

#[test]
fn scenario_dispatcher_two_buckets_sorted_ascending() {
    // @step Given spec/work-units.json contains two done units with estimate=5, iterations [1,2] and two done units with estimate=3, iterations [1,2]
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("A", "done", json!({ "estimate": 5, "iterations": 1 })),
            ("B", "done", json!({ "estimate": 5, "iterations": 2 })),
            ("C", "done", json!({ "estimate": 3, "iterations": 1 })),
            ("D", "done", json!({ "estimate": 3, "iterations": 2 })),
        ]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then patterns[0].points=3 and patterns[1].points=5
    let arr = data["patterns"].as_array().expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["points"].as_u64(), Some(3));
    assert_eq!(arr[1]["points"].as_u64(), Some(5));
}

#[test]
fn scenario_dispatcher_pattern_field_declaration_order() {
    // @step Given spec/work-units.json contains a single done unit with estimate=3 and iterations=1
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "done", json!({ "estimate": 3, "iterations": 1 }))]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));

    // @step Then the pattern object's field declaration order is points, expectedIterations, confidence
    let raw = &result.data;
    let expected = ["\"points\"", "\"expectedIterations\"", "\"confidence\""];
    let mut positions = Vec::new();
    for f in &expected {
        positions.push(raw.find(f).unwrap_or_else(|| panic!("missing {f}\n{raw}")));
    }
    for w in positions.windows(2) {
        assert!(
            w[0] < w[1],
            "field declaration order violated: {positions:?}\nraw:\n{raw}"
        );
    }
}

#[test]
fn scenario_dispatcher_malformed_work_units_json_yields_parse_error() {
    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), "{ not json");

    // @step When I dispatch query-estimation-guide against that project root with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
    assert!(!result.success);
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Failed to parse work-units.json"),
        "expected error containing 'Failed to parse work-units.json'; got: {err}"
    );
}

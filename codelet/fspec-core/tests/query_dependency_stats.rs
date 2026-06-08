#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/query-dependency-stats-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `query-dependency-stats` (RPC-257). Each scenario maps to exactly one
// #[test] function with @step comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "query-dependency-stats".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

/// Build a work-units.json string with the given (id, optional deps) entries.
/// `deps` carries the four dependency arrays as raw JSON fragments.
fn work_units_with(entries: &[(&str, Value)]) -> String {
    let mut wus = serde_json::Map::new();
    for (id, deps) in entries {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("status".into(), Value::String("backlog".to_string()));
        obj.insert("createdAt".into(), Value::String("x".to_string()));
        obj.insert("updatedAt".into(), Value::String("x".to_string()));
        if let Value::Object(extra) = deps {
            for (k, v) in extra {
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

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn returns_all_zero_stats_when_work_units_json_is_auto_created_in_empty_workspace() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch query-dependency-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let data = parse_data(&result.data);

    // @step Then the returned JSON has totalBlocks=0, totalBlockedBy=0, totalDependsOn=0, totalRelatesTo=0
    assert_eq!(data["totalBlocks"].as_u64(), Some(0));
    assert_eq!(data["totalBlockedBy"].as_u64(), Some(0));
    assert_eq!(data["totalDependsOn"].as_u64(), Some(0));
    assert_eq!(data["totalRelatesTo"].as_u64(), Some(0));

    // @step Then the returned JSON has workUnitsWithDependencies=0, workUnitsWithBlockers=0, workUnitsBlockingOthers=0, workUnitsWithSoftDependencies=0
    assert_eq!(data["workUnitsWithDependencies"].as_u64(), Some(0));
    assert_eq!(data["workUnitsWithBlockers"].as_u64(), Some(0));
    assert_eq!(data["workUnitsBlockingOthers"].as_u64(), Some(0));
    assert_eq!(data["workUnitsWithSoftDependencies"].as_u64(), Some(0));

    // @step Then the returned JSON has averageDependenciesPerUnit=0 and maxDependencyChainDepth=0
    assert_eq!(data["averageDependenciesPerUnit"].as_u64(), Some(0));
    assert_eq!(data["maxDependencyChainDepth"].as_u64(), Some(0));

    // @step Then spec/work-units.json exists after the call (auto-created by ensure_work_units_file)
    assert!(
        tmp.path().join("spec/work-units.json").exists(),
        "spec/work-units.json must be auto-created by ensure_work_units_file"
    );
}

#[test]
fn aggregates_totals_across_all_four_dependency_arrays() {
    // @step Given spec/work-units.json contains A whose blocks=['B'], blockedBy=[], dependsOn=['C'], relatesTo=['D']
    // @step Given spec/work-units.json also contains B (no dependency fields), C (no dependency fields), D (no dependency fields)
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            (
                "A",
                json!({
                    "blocks": ["B"],
                    "blockedBy": [],
                    "dependsOn": ["C"],
                    "relatesTo": ["D"]
                }),
            ),
            ("B", json!({})),
            ("C", json!({})),
            ("D", json!({})),
        ]),
    );

    // @step When I dispatch query-dependency-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the returned JSON has totalBlocks=1, totalBlockedBy=0, totalDependsOn=1, totalRelatesTo=1
    assert_eq!(data["totalBlocks"].as_u64(), Some(1));
    assert_eq!(data["totalBlockedBy"].as_u64(), Some(0));
    assert_eq!(data["totalDependsOn"].as_u64(), Some(1));
    assert_eq!(data["totalRelatesTo"].as_u64(), Some(1));

    // @step Then the returned JSON has workUnitsBlockingOthers=1, workUnitsWithBlockers=0, workUnitsWithSoftDependencies=1
    assert_eq!(data["workUnitsBlockingOthers"].as_u64(), Some(1));
    assert_eq!(data["workUnitsWithBlockers"].as_u64(), Some(0));
    assert_eq!(data["workUnitsWithSoftDependencies"].as_u64(), Some(1));

    // @step Then the returned JSON has workUnitsWithDependencies=1
    assert_eq!(data["workUnitsWithDependencies"].as_u64(), Some(1));
}

#[test]
fn two_unit_a_blocks_b_graph_reports_chain_depth_1() {
    // @step Given spec/work-units.json contains A with blocks=['B'] and B with no dependency fields
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", json!({"blocks": ["B"]})), ("B", json!({}))]),
    );

    // @step When I dispatch query-dependency-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the returned JSON has totalBlocks=1
    assert_eq!(data["totalBlocks"].as_u64(), Some(1));

    // @step Then the returned JSON has maxDependencyChainDepth=1
    assert_eq!(data["maxDependencyChainDepth"].as_u64(), Some(1));

    // @step Then the returned JSON has averageDependenciesPerUnit=0.5
    assert_eq!(data["averageDependenciesPerUnit"].as_f64(), Some(0.5));
}

#[test]
fn three_unit_linear_chain_reports_chain_depth_2() {
    // @step Given spec/work-units.json contains A with blocks=['B'], B with blocks=['C'], C with no dependency fields
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("A", json!({"blocks": ["B"]})),
            ("B", json!({"blocks": ["C"]})),
            ("C", json!({})),
        ]),
    );

    // @step When I dispatch query-dependency-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the returned JSON has totalBlocks=2
    assert_eq!(data["totalBlocks"].as_u64(), Some(2));

    // @step Then the returned JSON has maxDependencyChainDepth=2
    assert_eq!(data["maxDependencyChainDepth"].as_u64(), Some(2));
}

#[test]
fn self_cycle_on_blocks_yields_chain_depth_1_not_infinite_recursion() {
    // @step Given spec/work-units.json contains A with blocks=['A']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", json!({"blocks": ["A"]}))]),
    );

    // @step When I dispatch query-dependency-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the returned JSON has totalBlocks=1
    assert_eq!(data["totalBlocks"].as_u64(), Some(1));

    // @step Then the returned JSON has maxDependencyChainDepth=1
    assert_eq!(data["maxDependencyChainDepth"].as_u64(), Some(1));
}

#[test]
fn blocks_pointing_to_missing_work_unit_still_contributes_plus_1_to_chain_depth() {
    // @step Given spec/work-units.json contains A with blocks=['NONEXISTENT']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", json!({"blocks": ["NONEXISTENT"]}))]),
    );

    // @step When I dispatch query-dependency-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the returned JSON has totalBlocks=1
    assert_eq!(data["totalBlocks"].as_u64(), Some(1));

    // @step Then the returned JSON has maxDependencyChainDepth=1
    assert_eq!(data["maxDependencyChainDepth"].as_u64(), Some(1));
}

#[test]
fn empty_blocks_array_does_not_bump_work_units_blocking_others() {
    // @step Given spec/work-units.json contains A with blocks=[] (empty array)
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", json!({"blocks": []}))]),
    );

    // @step When I dispatch query-dependency-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the returned JSON has totalBlocks=0
    assert_eq!(data["totalBlocks"].as_u64(), Some(0));

    // @step Then the returned JSON has workUnitsBlockingOthers=0
    assert_eq!(data["workUnitsBlockingOthers"].as_u64(), Some(0));

    // @step Then the returned JSON has workUnitsWithDependencies=0
    assert_eq!(data["workUnitsWithDependencies"].as_u64(), Some(0));

    // @step Then the returned JSON has maxDependencyChainDepth=0
    assert_eq!(data["maxDependencyChainDepth"].as_u64(), Some(0));
}

#[test]
fn work_units_with_dependencies_counts_each_unit_only_once_even_with_multiple_populated_arrays() {
    // @step Given spec/work-units.json contains a single work unit A whose blocks=['X'], blockedBy=['Y'], dependsOn=['Z'], relatesTo=['W']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[(
            "A",
            json!({
                "blocks": ["X"],
                "blockedBy": ["Y"],
                "dependsOn": ["Z"],
                "relatesTo": ["W"]
            }),
        )]),
    );

    // @step When I dispatch query-dependency-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step Then the returned JSON has totalBlocks=1, totalBlockedBy=1, totalDependsOn=1, totalRelatesTo=1
    assert_eq!(data["totalBlocks"].as_u64(), Some(1));
    assert_eq!(data["totalBlockedBy"].as_u64(), Some(1));
    assert_eq!(data["totalDependsOn"].as_u64(), Some(1));
    assert_eq!(data["totalRelatesTo"].as_u64(), Some(1));

    // @step Then the returned JSON has workUnitsBlockingOthers=1, workUnitsWithBlockers=1, workUnitsWithSoftDependencies=1
    assert_eq!(data["workUnitsBlockingOthers"].as_u64(), Some(1));
    assert_eq!(data["workUnitsWithBlockers"].as_u64(), Some(1));
    assert_eq!(data["workUnitsWithSoftDependencies"].as_u64(), Some(1));

    // @step Then the returned JSON has workUnitsWithDependencies=1
    assert_eq!(data["workUnitsWithDependencies"].as_u64(), Some(1));
}

#[test]
fn integer_valued_average_serializes_without_a_decimal_point() {
    // @step Given spec/work-units.json contains three units where total dependency count divides exactly to integer 1
    // 3 units, 3 total deps → 1.0 average
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("A", json!({"blocks": ["B", "C"]})),
            ("B", json!({"blockedBy": ["A"]})),
            ("C", json!({})),
        ]),
    );

    // @step When I dispatch query-dependency-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the returned JSON text contains the line '  "averageDependenciesPerUnit": 1,'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  \"averageDependenciesPerUnit\": 1,"),
        "expected exact integer-formatted average line; got:\n{}",
        result.data
    );

    // @step Then the returned JSON text does NOT contain the substring 'averageDependenciesPerUnit": 1.0'
    assert!(
        !result.data.contains("averageDependenciesPerUnit\": 1.0"),
        "average must NOT serialize as 1.0; got:\n{}",
        result.data
    );
}

#[test]
fn non_integer_average_serializes_as_decimal() {
    // @step Given spec/work-units.json contains exactly two units, one with blocks=['X'] and one with no dependency fields
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", json!({"blocks": ["X"]})), ("B", json!({}))]),
    );

    // @step When I dispatch query-dependency-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the returned JSON text contains the line '  "averageDependenciesPerUnit": 0.5,'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  \"averageDependenciesPerUnit\": 0.5,"),
        "expected exact decimal-formatted average line; got:\n{}",
        result.data
    );
}

#[test]
fn json_field_order_matches_ts_declaration_order_exactly() {
    // @step Given spec/work-units.json contains a single unit A with blocks=['X']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", json!({"blocks": ["X"]}))]),
    );

    // @step When I dispatch query-dependency-stats with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the returned JSON's field declaration order is totalBlocks, totalBlockedBy, totalDependsOn, totalRelatesTo, workUnitsWithDependencies, workUnitsWithBlockers, workUnitsBlockingOthers, workUnitsWithSoftDependencies, averageDependenciesPerUnit, maxDependencyChainDepth
    let expected = [
        "\"totalBlocks\"",
        "\"totalBlockedBy\"",
        "\"totalDependsOn\"",
        "\"totalRelatesTo\"",
        "\"workUnitsWithDependencies\"",
        "\"workUnitsWithBlockers\"",
        "\"workUnitsBlockingOthers\"",
        "\"workUnitsWithSoftDependencies\"",
        "\"averageDependenciesPerUnit\"",
        "\"maxDependencyChainDepth\"",
    ];
    let mut positions: Vec<usize> = Vec::new();
    for field in &expected {
        let pos = result
            .data
            .find(field)
            .unwrap_or_else(|| panic!("field {field} missing from output:\n{}", result.data));
        positions.push(pos);
    }
    for w in positions.windows(2) {
        assert!(
            w[0] < w[1],
            "field order violated. positions: {positions:?}\noutput:\n{}",
            result.data
        );
    }
}

#[test]
fn escalates_malformed_work_units_json_as_structured_parse_error() {
    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), "{ not json");

    // @step When I dispatch query-dependency-stats against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Failed to parse work-units.json"),
        "error message missing canonical substring: {msg}"
    );
}

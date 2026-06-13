#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/query-estimate-accuracy-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `query-estimate-accuracy`
// (RPC-258). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "query-estimate-accuracy".to_string(),
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

/// Helper to assemble a `spec/work-units.json` body from a list of `(id, json-fragment)`
/// tuples preserving insertion order. Each fragment is the JSON object body
/// (already including the surrounding braces) for the work unit.
fn raw_work_units(entries: &[(&str, &str)]) -> String {
    let mut out = String::from("{\n  \"workUnits\": {");
    for (i, (id, body)) in entries.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("\n    \"{id}\": {body}"));
    }
    out.push_str("\n  }\n}\n");
    out
}

// ---------- scenarios ----------

#[test]
fn returns_empty_by_story_points_when_spec_does_not_exist() {
    // Scenario: Returns an empty byStoryPoints object when spec/ does not exist and does not auto-create files

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch the query-estimate-accuracy command against that project root with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true with a payload whose byStoryPoints object is empty
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    assert_eq!(
        data["byStoryPoints"]
            .as_object()
            .map(serde_json::Map::is_empty),
        Some(true),
        "expected empty byStoryPoints object, got {}",
        result.data
    );

    // @step And spec/work-units.json does not exist after the call
    assert!(
        !tmp.path().join("spec/work-units.json").exists(),
        "query-estimate-accuracy must NOT auto-create spec/work-units.json"
    );
}

#[test]
fn escalates_malformed_work_units_json_as_wrapped_error() {
    // Scenario: Escalates malformed work-units.json as a structured wrapped error

    // @step Given spec/work-units.json exists but contains invalid JSON syntax
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), "{ not valid json");

    // @step When I dispatch the query-estimate-accuracy command against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");

    // @step And the error message contains the substring 'Failed to query estimate accuracy:'
    assert!(
        msg.contains("Failed to query estimate accuracy:"),
        "missing canonical wrapper prefix; got: {msg}"
    );

    // @step And the error message contains the substring 'Failed to parse work-units.json'
    assert!(
        msg.contains("Failed to parse work-units.json"),
        "missing inner parse-error substring; got: {msg}"
    );
}

#[test]
fn single_work_unit_returns_estimated_actual_comparison() {
    // Scenario: Single work-unit query returns estimated, actual, and comparison fields

    // @step Given spec/work-units.json contains the work unit AUTH-001 with estimate=5, iterations=2, and status='done'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &raw_work_units(&[(
            "AUTH-001",
            r#"{"id":"AUTH-001","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":5,"iterations":2}"#,
        )]),
    );

    // @step When I dispatch query-estimate-accuracy with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step And the payload field 'estimated' equals '5 points'
    assert_eq!(data["estimated"].as_str(), Some("5 points"));

    // @step And the payload field 'actual' equals '0 tokens, 2 iterations'
    assert_eq!(data["actual"].as_str(), Some("0 tokens, 2 iterations"));

    // @step And the payload field 'comparison' equals 'Within expected range'
    assert_eq!(data["comparison"].as_str(), Some("Within expected range"));
}

#[test]
fn single_work_unit_unknown_id_returns_wrapped_not_found_error() {
    // Scenario: Single work-unit query for unknown id returns wrapped not-found error

    // @step Given spec/work-units.json contains no work unit with id 'MISSING-999'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), r#"{ "workUnits": {} }"#);

    // @step When I dispatch query-estimate-accuracy with workUnitId='MISSING-999'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "MISSING-999" })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "{result:?}");
    let msg = result.error.as_ref().expect("error message");

    // @step And the error message contains the substring 'Failed to query estimate accuracy:'
    assert!(
        msg.contains("Failed to query estimate accuracy:"),
        "missing wrapper prefix; got: {msg}"
    );

    // @step And the error message contains the substring 'Work unit MISSING-999 not found'
    assert!(
        msg.contains("Work unit MISSING-999 not found"),
        "missing 'Work unit MISSING-999 not found' substring; got: {msg}"
    );
}

#[test]
fn single_work_unit_defaults_missing_estimate_and_iterations_to_zero() {
    // Scenario: Single work-unit query defaults missing estimate and iterations to zero

    // @step Given spec/work-units.json contains the work unit BUG-001 with no estimate field and no iterations field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &raw_work_units(&[(
            "BUG-001",
            r#"{"id":"BUG-001","title":"t","status":"done","createdAt":"x","updatedAt":"x"}"#,
        )]),
    );

    // @step When I dispatch query-estimate-accuracy with workUnitId='BUG-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "BUG-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step And the payload field 'estimated' equals '0 points'
    assert_eq!(data["estimated"].as_str(), Some("0 points"));

    // @step And the payload field 'actual' equals '0 tokens, 0 iterations'
    assert_eq!(data["actual"].as_str(), Some("0 tokens, 0 iterations"));

    // @step And the payload field 'comparison' equals 'Within expected range'
    assert_eq!(data["comparison"].as_str(), Some("Within expected range"));
}

#[test]
fn single_work_unit_reads_iterations_from_metrics_field() {
    // Scenario: Single work-unit query reads iterations from metrics.iterations when root-level iterations is undefined

    // @step Given spec/work-units.json contains the work unit AUTH-002 with no root-level iterations field but with metrics.iterations=7
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &raw_work_units(&[(
            "AUTH-002",
            r#"{"id":"AUTH-002","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":3,"metrics":{"iterations":7}}"#,
        )]),
    );

    // @step When I dispatch query-estimate-accuracy with workUnitId='AUTH-002' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-002", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);

    // @step And the payload field 'actual' equals '0 tokens, 7 iterations'
    assert_eq!(data["actual"].as_str(), Some("0 tokens, 7 iterations"));
}

#[test]
fn all_completed_aggregation_buckets_by_story_point_with_avg_iterations() {
    // Scenario: All-completed aggregation buckets by story point with averaged iterations

    // @step Given spec/work-units.json contains five done work units AUTH-001 (estimate=1 iterations=1), AUTH-002 (estimate=1 iterations=2), AUTH-003 (estimate=3 iterations=2), AUTH-004 (estimate=3 iterations=3), AUTH-005 (estimate=5 iterations=2)
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &raw_work_units(&[
            (
                "AUTH-001",
                r#"{"id":"AUTH-001","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":1,"iterations":1}"#,
            ),
            (
                "AUTH-002",
                r#"{"id":"AUTH-002","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":1,"iterations":2}"#,
            ),
            (
                "AUTH-003",
                r#"{"id":"AUTH-003","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":3,"iterations":2}"#,
            ),
            (
                "AUTH-004",
                r#"{"id":"AUTH-004","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":3,"iterations":3}"#,
            ),
            (
                "AUTH-005",
                r#"{"id":"AUTH-005","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":5,"iterations":2}"#,
            ),
        ]),
    );

    // @step When I dispatch query-estimate-accuracy with format='json' and no workUnitId
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    let bsp = &data["byStoryPoints"];

    // @step And the byStoryPoints entry for '1' has avgIterations=1.5 and samples=2
    assert_eq!(bsp["1"]["avgIterations"].as_f64(), Some(1.5));
    assert_eq!(bsp["1"]["samples"].as_u64(), Some(2));

    // @step And the byStoryPoints entry for '3' has avgIterations=2.5 and samples=2
    assert_eq!(bsp["3"]["avgIterations"].as_f64(), Some(2.5));
    assert_eq!(bsp["3"]["samples"].as_u64(), Some(2));

    // @step And the byStoryPoints entry for '5' has avgIterations=2 and samples=1
    assert_eq!(bsp["5"]["avgIterations"].as_f64(), Some(2.0));
    assert_eq!(bsp["5"]["samples"].as_u64(), Some(1));
}

#[test]
fn all_completed_aggregation_excludes_units_missing_estimate_or_iterations() {
    // Scenario: All-completed aggregation excludes work units missing estimate or iterations

    // @step Given spec/work-units.json contains the done work unit AUTH-100 with estimate=3 but no iterations field
    // @step And spec/work-units.json also contains the done work unit AUTH-101 with iterations=4 but no estimate field
    // @step And spec/work-units.json also contains the done work unit AUTH-102 with estimate=2 and iterations=3
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &raw_work_units(&[
            (
                "AUTH-100",
                r#"{"id":"AUTH-100","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":3}"#,
            ),
            (
                "AUTH-101",
                r#"{"id":"AUTH-101","title":"t","status":"done","createdAt":"x","updatedAt":"x","iterations":4}"#,
            ),
            (
                "AUTH-102",
                r#"{"id":"AUTH-102","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":2,"iterations":3}"#,
            ),
        ]),
    );

    // @step When I dispatch query-estimate-accuracy with format='json' and no workUnitId
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    let bsp = data["byStoryPoints"]
        .as_object()
        .expect("byStoryPoints object");

    // @step Then the byStoryPoints object has exactly one key '2'
    assert_eq!(bsp.len(), 1, "expected exactly one key, got {bsp:?}");
    assert!(bsp.contains_key("2"), "expected key '2', got {bsp:?}");

    // @step And the byStoryPoints entry for '2' has avgIterations=3 and samples=1
    assert_eq!(bsp["2"]["avgIterations"].as_f64(), Some(3.0));
    assert_eq!(bsp["2"]["samples"].as_u64(), Some(1));
}

#[test]
fn by_prefix_true_groups_by_id_prefix_and_reports_strings() {
    // Scenario: byPrefix=true groups by id prefix and reports avgAccuracy and recommendation strings

    // @step Given spec/work-units.json contains AUTH-001 (estimate=5 iterations=2 status=done), AUTH-002 (estimate=3 iterations=4 status=done), SEC-001 (estimate=5 iterations=3 status=done)
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &raw_work_units(&[
            (
                "AUTH-001",
                r#"{"id":"AUTH-001","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":5,"iterations":2}"#,
            ),
            (
                "AUTH-002",
                r#"{"id":"AUTH-002","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":3,"iterations":4}"#,
            ),
            (
                "SEC-001",
                r#"{"id":"SEC-001","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":5,"iterations":3}"#,
            ),
        ]),
    );

    // @step When I dispatch query-estimate-accuracy with byPrefix=true and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "byPrefix": true, "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");
    let data = parse_data(&result.data);
    let bp = &data["byPrefix"];

    // @step And the byPrefix entry for 'AUTH' has avgAccuracy='3.0 avg iterations' and recommendation='2 samples'
    assert_eq!(
        bp["AUTH"]["avgAccuracy"].as_str(),
        Some("3.0 avg iterations")
    );
    assert_eq!(bp["AUTH"]["recommendation"].as_str(), Some("2 samples"));

    // @step And the byPrefix entry for 'SEC' has avgAccuracy='3.0 avg iterations' and recommendation='1 sample'
    assert_eq!(
        bp["SEC"]["avgAccuracy"].as_str(),
        Some("3.0 avg iterations")
    );
    assert_eq!(bp["SEC"]["recommendation"].as_str(), Some("1 sample"));
}

#[test]
fn by_story_points_key_iteration_order_follows_numeric_ascending() {
    // Scenario: byStoryPoints key iteration order follows ascending numeric order
    // (TS-parity: JavaScript Object.entries enumerates integer-like keys in
    // ascending numeric order, regardless of insertion order)

    // @step Given spec/work-units.json contains the done work units ZED-001 (estimate=5 iterations=1), AAA-001 (estimate=1 iterations=1), MID-001 (estimate=3 iterations=1) registered in that order
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &raw_work_units(&[
            (
                "ZED-001",
                r#"{"id":"ZED-001","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":5,"iterations":1}"#,
            ),
            (
                "AAA-001",
                r#"{"id":"AAA-001","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":1,"iterations":1}"#,
            ),
            (
                "MID-001",
                r#"{"id":"MID-001","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":3,"iterations":1}"#,
            ),
        ]),
    );

    // @step When I dispatch query-estimate-accuracy with format='json' and no workUnitId
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the byStoryPoints keys appear in ascending numeric order '1', '3', '5' in the serialized payload
    let one = result.data.find("\"1\"").expect("'1' key present");
    let three = result.data.find("\"3\"").expect("'3' key present");
    let five = result.data.find("\"5\"").expect("'5' key present");
    assert!(
        one < three && three < five,
        "expected ascending numeric order 1 < 3 < 5; got 1={one} 3={three} 5={five}\n{}",
        result.data
    );
}

#[test]
fn json_payload_is_pretty_printed_with_canonical_field_order() {
    // Scenario: JSON dispatcher payload is pretty-printed with 2-space indent and canonical field order

    // @step Given spec/work-units.json contains the done work unit AUTH-001 (estimate=5 iterations=2)
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &raw_work_units(&[(
            "AUTH-001",
            r#"{"id":"AUTH-001","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":5,"iterations":2}"#,
        )]),
    );

    // @step When I dispatch query-estimate-accuracy with byPrefix=true and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "byPrefix": true, "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the payload string contains a line that starts with two spaces followed by '"byStoryPoints"'
    assert!(
        result
            .data
            .lines()
            .any(|l| l.starts_with("  \"byStoryPoints\"")),
        "expected a line starting with two-space indent + \"byStoryPoints\"; got:\n{}",
        result.data
    );

    // @step And the payload string contains a line that starts with two spaces followed by '"byPrefix"'
    assert!(
        result.data.lines().any(|l| l.starts_with("  \"byPrefix\"")),
        "expected a line starting with two-space indent + \"byPrefix\"; got:\n{}",
        result.data
    );

    // @step And the byStoryPoints field appears before the byPrefix field in the payload string
    let bsp_pos = result
        .data
        .find("\"byStoryPoints\"")
        .expect("byStoryPoints present");
    let bp_pos = result.data.find("\"byPrefix\"").expect("byPrefix present");
    assert!(
        bsp_pos < bp_pos,
        "expected byStoryPoints before byPrefix; got bsp={bsp_pos} bp={bp_pos}\n{}",
        result.data
    );
}

#[test]
fn all_completed_against_missing_work_units_json_returns_empty() {
    // Scenario: All-completed aggregation against missing work-units.json returns empty byStoryPoints

    // @step Given spec/work-units.json does not exist in the project root
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch query-estimate-accuracy with format='json' and no workUnitId
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the payload field 'byStoryPoints' is an empty object
    let data = parse_data(&result.data);
    assert_eq!(
        data["byStoryPoints"]
            .as_object()
            .map(serde_json::Map::is_empty),
        Some(true),
        "expected empty byStoryPoints object; got:\n{}",
        result.data
    );

    // @step And spec/work-units.json was NOT created in the project root
    assert!(
        !tmp.path().join("spec/work-units.json").exists(),
        "query-estimate-accuracy must NOT auto-create spec/work-units.json"
    );
}

#[test]
fn both_invocation_paths_call_the_same_fspec_core_run_function() {
    // Scenario: Both invocation paths call the same fspec_core::commands::query_estimate_accuracy::run function

    // @step Given a project root whose spec/work-units.json contains the done work unit AUTH-001 (estimate=5 iterations=2)
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &raw_work_units(&[(
            "AUTH-001",
            r#"{"id":"AUTH-001","title":"t","status":"done","createdAt":"x","updatedAt":"x","estimate":5,"iterations":2}"#,
        )]),
    );

    // @step When I dispatch query-estimate-accuracy through fspec_core::dispatch::dispatch_command with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step And the dispatcher payload contains a byStoryPoints entry for '5' with samples=1
    let data = parse_data(&result.data);
    assert_eq!(data["byStoryPoints"]["5"]["samples"].as_u64(), Some(1));
    assert_eq!(
        data["byStoryPoints"]["5"]["avgIterations"].as_f64(),
        Some(2.0)
    );

    // @step And the CLI bridge module codelet/fspec/src/query_estimate_accuracy.rs contains no inline aggregation or rendering logic
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("codelet/")
        .join("fspec/src/query_estimate_accuracy.rs");
    // The bridge module may not yet exist in Phase B — the source-shape
    // assertion only fires if the file is present (Phase C will create it
    // and re-run this scenario as part of the green-phase test pass).
    if bridge_path.exists() {
        let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
        for forbidden in [
            "avgIterations",
            "byStoryPoints",
            "byPrefix",
            "Estimation Accuracy Report",
            "By Story Points",
        ] {
            assert!(
                !bridge_src.contains(forbidden),
                "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
            );
        }
    }
}

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/export-example-map-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `export-example-map`
// (RPC-228) through the LLM-facing dispatcher front door. Each scenario maps
// to exactly one #[test] function with @step comments mirroring the Gherkin
// steps verbatim.
//
// Red phase: until RPC-228 is wired into run_ported (Phase C), the dispatcher
// routes `export-example-map` to the NotYetPorted stub, so every success
// assertion below fails. That is the expected red-phase signal.

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
        command: "export-example-map".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

/// Build a work-units.json string with the given (id, extra-fields) entries.
fn work_units_with(entries: &[(&str, Value)]) -> String {
    let mut wus = serde_json::Map::new();
    for (id, extra) in entries {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("status".into(), Value::String("backlog".to_string()));
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
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn export_a_work_unit_with_full_example_mapping_data() {
    // @step Given a work units store where AUTH-001 has one rule, one example, one question, and one assumption
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[(
            "AUTH-001",
            json!({
                "rules": [{ "id": 0, "text": "r1", "deleted": false, "createdAt": "x" }],
                "examples": [{ "id": 0, "text": "e1", "deleted": false, "createdAt": "x" }],
                "questions": [{ "id": 0, "text": "q1", "deleted": false, "selected": false, "createdAt": "x" }],
                "assumptions": ["a1"]
            }),
        )]),
    );

    // @step When I export the example map for AUTH-001 to emap.json
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "file": "emap.json" }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the written JSON has fields workUnitId, title, rules, examples, questions, and assumptions in that order
    let written = fs::read_to_string(tmp.path().join("emap.json")).expect("read emap.json");
    let expected = [
        "\"workUnitId\"",
        "\"title\"",
        "\"rules\"",
        "\"examples\"",
        "\"questions\"",
        "\"assumptions\"",
    ];
    let mut positions: Vec<usize> = Vec::new();
    for field in &expected {
        positions.push(
            written
                .find(field)
                .unwrap_or_else(|| panic!("missing {field} in:\n{written}")),
        );
    }
    for w in positions.windows(2) {
        assert!(
            w[0] < w[1],
            "field order violated: {positions:?}\n{written}"
        );
    }

    // @step And the rules, examples, questions, and assumptions arrays each contain their single item verbatim
    let parsed: Value = serde_json::from_str(&written).expect("emap.json is JSON");
    assert_eq!(parsed["workUnitId"].as_str(), Some("AUTH-001"));
    assert_eq!(parsed["title"].as_str(), Some("title AUTH-001"));
    assert_eq!(parsed["rules"].as_array().map(Vec::len), Some(1));
    assert_eq!(parsed["rules"][0]["text"].as_str(), Some("r1"));
    assert_eq!(parsed["examples"].as_array().map(Vec::len), Some(1));
    assert_eq!(parsed["examples"][0]["text"].as_str(), Some("e1"));
    assert_eq!(parsed["questions"].as_array().map(Vec::len), Some(1));
    assert_eq!(parsed["questions"][0]["selected"].as_bool(), Some(false));
    assert_eq!(parsed["assumptions"][0].as_str(), Some("a1"));

    // @step And the returned message is "✓ Exported to emap.json"
    assert_eq!(result.data, "✓ Exported to emap.json");
}

#[test]
fn export_a_work_unit_with_no_example_mapping_data() {
    // @step Given a work units store where AUTH-002 has no rules, examples, questions, or assumptions
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with(&[("AUTH-002", json!({}))]));

    // @step When I export the example map for AUTH-002 to emap2.json
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-002", "file": "emap2.json" }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the written JSON has rules, examples, questions, and assumptions all as empty arrays
    let written = fs::read_to_string(tmp.path().join("emap2.json")).expect("read emap2.json");
    let parsed: Value = serde_json::from_str(&written).expect("emap2.json is JSON");
    assert_eq!(parsed["rules"], json!([]));
    assert_eq!(parsed["examples"], json!([]));
    assert_eq!(parsed["questions"], json!([]));
    assert_eq!(parsed["assumptions"], json!([]));
}

#[test]
fn export_to_a_nested_output_path_creates_parent_directories() {
    // @step Given a work units store containing AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with(&[("AUTH-001", json!({}))]));

    // @step When I export the example map for AUTH-001 to out/maps/emap.json
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "file": "out/maps/emap.json" }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the parent directory out/maps is created and the file is written
    assert!(
        tmp.path().join("out/maps").is_dir(),
        "out/maps directory must be created"
    );
    assert!(
        tmp.path().join("out/maps/emap.json").is_file(),
        "out/maps/emap.json must be written"
    );
}

#[test]
fn export_a_work_unit_that_does_not_exist_fails() {
    // @step Given a work units store that does not contain NOPE-999
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with(&[("AUTH-001", json!({}))]));

    // @step When I export the example map for NOPE-999 to out.json
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "NOPE-999", "file": "out.json" }),
    ));

    // @step Then the run returns an error containing "Work unit 'NOPE-999' does not exist"
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Work unit 'NOPE-999' does not exist"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn export_with_a_malformed_work_units_file_escalates_a_parse_error() {
    // @step Given a spec/work-units.json file that is not valid JSON
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), "{ not json");

    // @step When I export the example map for AUTH-001 to out.json
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "file": "out.json" }),
    ));

    // @step Then the run returns an error containing "Failed to parse work-units.json"
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Failed to parse work-units.json"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn dispatcher_and_core_produce_identical_file_content() {
    // @step Given a work units store containing AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[(
            "AUTH-001",
            json!({
                "rules": [{ "id": 0, "text": "r1", "deleted": false, "createdAt": "x" }],
                "assumptions": ["a1"]
            }),
        )]),
    );

    // @step When I export the example map for AUTH-001 via the core run function
    let first = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "file": "a.json" }),
    ));
    let second = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "file": "b.json" }),
    ));
    assert!(first.success && second.success, "{first:?} {second:?}");

    // @step Then the written file content is the same as exporting via the dispatcher path
    let a = fs::read_to_string(tmp.path().join("a.json")).expect("read a.json");
    let b = fs::read_to_string(tmp.path().join("b.json")).expect("read b.json");
    assert_eq!(
        a, b,
        "both export paths must produce identical file content"
    );
}

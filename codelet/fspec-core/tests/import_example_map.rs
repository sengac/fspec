#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/import-example-map-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `import-example-map`
// (RPC-238) through the LLM-facing dispatcher front door. Each scenario maps
// to exactly one #[test] function with @step comments mirroring the Gherkin
// steps verbatim.
//
// import-example-map is the inverse of export-example-map (RPC-228): it reads
// an Example-Mapping JSON file and APPENDS its arrays to a work unit.
//
// Red phase: until RPC-238 is wired into run_ported (Phase C), the dispatcher
// routes `import-example-map` to the NotYetPorted stub, so every success
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
        command: "import-example-map".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn write_import_file(project_root: &Path, name: &str, value: &Value) {
    fs::write(
        project_root.join(name),
        serde_json::to_string_pretty(value).unwrap(),
    )
    .expect("write import file");
}

fn read_work_units(project_root: &Path) -> Value {
    let raw =
        fs::read_to_string(project_root.join("spec/work-units.json")).expect("read work-units");
    serde_json::from_str(&raw).expect("work-units is JSON")
}

/// Build a work-units.json string with one work unit at `status`, optionally
/// seeded with extra fields (e.g. an existing `rules` array).
fn store_with(id: &str, status: &str, extra: Value) -> String {
    let mut obj = serde_json::Map::new();
    obj.insert("id".into(), Value::String(id.to_string()));
    obj.insert("title".into(), Value::String(format!("title {id}")));
    obj.insert("status".into(), Value::String(status.to_string()));
    obj.insert("createdAt".into(), Value::String("x".to_string()));
    obj.insert("updatedAt".into(), Value::String("x".to_string()));
    if let Value::Object(map) = extra {
        for (k, v) in map {
            obj.insert(k, v);
        }
    }
    let mut wus = serde_json::Map::new();
    wus.insert(id.to_string(), Value::Object(obj));
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
fn import_full_example_mapping_data_into_a_specifying_work_unit() {
    // @step Given a work units store where AUTH-001 is in specifying state with no example map data
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &store_with("AUTH-001", "specifying", json!({})));

    // @step And an import file with two rules, three examples, one question, and zero assumptions
    write_import_file(
        tmp.path(),
        "emap.json",
        &json!({
            "rules": ["r1", "r2"],
            "examples": ["e1", "e2", "e3"],
            "questions": ["q1"],
            "assumptions": []
        }),
    );

    // @step When I import the example map from the file into AUTH-001
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "file": "emap.json" }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the AUTH-001 rules, examples, and questions arrays contain the imported items
    let data = read_work_units(tmp.path());
    let wu = &data["workUnits"]["AUTH-001"];
    assert_eq!(wu["rules"].as_array().map(Vec::len), Some(2));
    assert_eq!(wu["examples"].as_array().map(Vec::len), Some(3));
    assert_eq!(wu["questions"].as_array().map(Vec::len), Some(1));

    // @step And the returned message is "✓ Imported 6 items: 2 rules, 3 examples, 1 questions, 0 assumptions"
    assert_eq!(
        result.data,
        "✓ Imported 6 items: 2 rules, 3 examples, 1 questions, 0 assumptions"
    );
}

#[test]
fn import_appends_to_existing_example_mapping_arrays() {
    // @step Given a work units store where AUTH-001 is in specifying state with one existing rule
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &store_with("AUTH-001", "specifying", json!({ "rules": ["existing"] })),
    );

    // @step And an import file with two rules
    write_import_file(tmp.path(), "emap.json", &json!({ "rules": ["r1", "r2"] }));

    // @step When I import the example map from the file into AUTH-001
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "file": "emap.json" }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the AUTH-001 rules array holds the existing rule followed by the two imported rules
    let data = read_work_units(tmp.path());
    let rules = data["workUnits"]["AUTH-001"]["rules"]
        .as_array()
        .expect("rules array");
    let texts: Vec<&str> = rules.iter().filter_map(Value::as_str).collect();
    assert_eq!(texts, vec!["existing", "r1", "r2"]);
}

#[test]
fn import_a_file_with_only_one_category_leaves_the_others_untouched() {
    // @step Given a work units store where AUTH-001 is in specifying state
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &store_with("AUTH-001", "specifying", json!({})));

    // @step And an import file containing only examples
    write_import_file(
        tmp.path(),
        "emap.json",
        &json!({ "examples": ["e1", "e2"] }),
    );

    // @step When I import the example map from the file into AUTH-001
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "file": "emap.json" }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then only the examples count is non-zero and the rules, questions, and assumptions arrays are unchanged
    assert_eq!(
        result.data,
        "✓ Imported 2 items: 0 rules, 2 examples, 0 questions, 0 assumptions"
    );
    let data = read_work_units(tmp.path());
    let wu = &data["workUnits"]["AUTH-001"];
    assert_eq!(wu["examples"].as_array().map(Vec::len), Some(2));
    assert!(
        wu.get("rules").map(Value::is_null).unwrap_or(true)
            || wu["rules"].as_array() == Some(&vec![]),
        "rules must remain untouched (absent or empty); got {:?}",
        wu.get("rules")
    );
}

#[test]
fn import_into_a_work_unit_that_does_not_exist_fails() {
    // @step Given a work units store that does not contain NOPE-999
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &store_with("AUTH-001", "specifying", json!({})));
    write_import_file(tmp.path(), "emap.json", &json!({ "rules": ["r1"] }));

    // @step When I import the example map from a file into NOPE-999
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "NOPE-999", "file": "emap.json" }),
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
fn import_into_a_work_unit_not_in_specifying_state_fails() {
    // @step Given a work units store where AUTH-009 is in done state
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &store_with("AUTH-009", "done", json!({})));
    write_import_file(tmp.path(), "emap.json", &json!({ "rules": ["r1"] }));

    // @step When I import the example map from a file into AUTH-009
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-009", "file": "emap.json" }),
    ));

    // @step Then the run returns an error containing "Can only import example mapping during discovery/specification phase. AUTH-009 is in 'done' state."
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains(
            "Can only import example mapping during discovery/specification phase. AUTH-009 is in 'done' state."
        ),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn dispatcher_and_core_append_identical_data() {
    // @step Given a work units store where AUTH-001 is in specifying state
    let tmp_a = TempDir::new().expect("tempdir a");
    let tmp_b = TempDir::new().expect("tempdir b");
    write_work_units(
        tmp_a.path(),
        &store_with("AUTH-001", "specifying", json!({})),
    );
    write_work_units(
        tmp_b.path(),
        &store_with("AUTH-001", "specifying", json!({})),
    );

    // @step And an import file with one rule and one example
    let import = json!({ "rules": ["r1"], "examples": ["e1"] });
    write_import_file(tmp_a.path(), "emap.json", &import);
    write_import_file(tmp_b.path(), "emap.json", &import);

    // @step When I import the example map for AUTH-001 via the core run function
    let first = dispatch_command(req(
        tmp_a.path(),
        json!({ "workUnitId": "AUTH-001", "file": "emap.json" }),
    ));
    let second = dispatch_command(req(
        tmp_b.path(),
        json!({ "workUnitId": "AUTH-001", "file": "emap.json" }),
    ));
    assert!(first.success && second.success, "{first:?} {second:?}");

    // @step Then the resulting work unit state matches importing via the dispatcher path
    let a = read_work_units(tmp_a.path());
    let b = read_work_units(tmp_b.path());
    assert_eq!(
        a["workUnits"]["AUTH-001"]["rules"],
        b["workUnits"]["AUTH-001"]["rules"]
    );
    assert_eq!(
        a["workUnits"]["AUTH-001"]["examples"],
        b["workUnits"]["AUTH-001"]["examples"]
    );
    assert_eq!(first.data, second.data);
}

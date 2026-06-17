#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/validate-spec-alignment-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of the
// `validate-spec-alignment` command (RPC-323). Each scenario maps to exactly
// one #[test] function with @step comments mirroring the Gherkin steps verbatim.
//
// Red phase: the `validate-spec-alignment` command is still a stub returning
// FspecCoreError::NotYetPorted, so every assertion below FAILS until Phase C
// wires the real implementation.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────── helpers ─────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "validate-spec-alignment".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

/// Write `spec/work-units.json` directly (validate-spec-alignment reads the
/// raw file via JSON.parse, not ensureWorkUnitsFile).
fn write_work_units(root: &Path, ids: &[&str]) {
    let spec = root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let mut wus = serde_json::Map::new();
    for id in ids {
        wus.insert(
            (*id).to_string(),
            json!({ "id": id, "title": format!("title {id}"), "status": "backlog" }),
        );
    }
    let payload = json!({ "workUnits": Value::Object(wus), "states": {} });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&payload).unwrap(),
    )
    .expect("write work-units.json");
}

fn write_feature(root: &Path, name: &str, content: &str) {
    let dir = root.join("spec/features");
    fs::create_dir_all(&dir).expect("mkdir spec/features");
    fs::write(dir.join(name), content).expect("write feature file");
}

fn parse(data: &str) -> Value {
    serde_json::from_str(data).expect("data must be JSON")
}

// ───────── scenarios ─────────

#[test]
fn reports_valid_when_at_least_one_scenario_is_tagged_with_the_work_unit_id() {
    // Scenario: Reports valid when at least one scenario is tagged with the work unit id

    // @step Given spec/work-units.json contains AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &["AUTH-001"]);

    // @step And a feature file has a line '@AUTH-001' immediately followed by a line starting with 'Scenario:'
    write_feature(
        tmp.path(),
        "auth.feature",
        "Feature: Auth\n\n  @AUTH-001\n  Scenario: logs in\n    Given x\n",
    );

    // @step When I dispatch validate-spec-alignment with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned JSON has valid=true
    let parsed = parse(&result.data);
    assert_eq!(parsed["valid"], json!(true), "got:\n{}", result.data);

    // @step And the returned JSON has no warnings field
    assert!(
        parsed.get("warnings").is_none(),
        "expected no warnings field, got:\n{}",
        result.data
    );
}

#[test]
fn reports_invalid_with_a_warning_when_no_scenario_is_tagged_with_the_work_unit_id() {
    // Scenario: Reports invalid with a warning when no scenario is tagged with the work unit id

    // @step Given spec/work-units.json contains AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &["AUTH-001"]);

    // @step And no feature scenario is tagged '@AUTH-001'
    write_feature(
        tmp.path(),
        "other.feature",
        "Feature: Other\n\n  @OTHER-001\n  Scenario: does stuff\n    Given x\n",
    );

    // @step When I dispatch validate-spec-alignment with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned JSON has valid=false
    let parsed = parse(&result.data);
    assert_eq!(parsed["valid"], json!(false), "got:\n{}", result.data);

    // @step And the returned JSON has warnings equal to ['No scenarios for AUTH-001']
    assert_eq!(
        parsed["warnings"],
        json!(["No scenarios for AUTH-001"]),
        "got:\n{}",
        result.data
    );
}

#[test]
fn errors_when_the_work_unit_does_not_exist() {
    // Scenario: Errors when the work unit does not exist

    // @step Given spec/work-units.json contains AUTH-001 but not MISSING-999
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &["AUTH-001"]);

    // @step When I dispatch validate-spec-alignment with workUnitId='MISSING-999'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "MISSING-999" })));

    // @step Then the dispatcher returns success=false with an error message containing 'Failed to validate spec alignment: Work unit MISSING-999 not found'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Failed to validate spec alignment: Work unit MISSING-999 not found"),
        "error message missing substring: {msg}"
    );
}

#[test]
fn a_missing_spec_features_directory_yields_zero_scenarios_and_an_invalid_result() {
    // Scenario: A missing spec/features directory yields zero scenarios and an invalid result

    // @step Given spec/work-units.json contains AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &["AUTH-001"]);

    // @step And the spec/features directory does not exist
    assert!(
        !tmp.path().join("spec/features").exists(),
        "precondition: spec/features must not exist"
    );

    // @step When I dispatch validate-spec-alignment with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned JSON has valid=false
    let parsed = parse(&result.data);
    assert_eq!(parsed["valid"], json!(false), "got:\n{}", result.data);

    // @step And the returned JSON has warnings equal to ['No scenarios for AUTH-001']
    assert_eq!(
        parsed["warnings"],
        json!(["No scenarios for AUTH-001"]),
        "got:\n{}",
        result.data
    );
}

#[test]
fn a_tag_substring_on_a_line_not_followed_by_scenario_does_not_count() {
    // Scenario: A tag substring on a line not followed by Scenario does not count

    // @step Given spec/work-units.json contains AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &["AUTH-001"]);

    // @step And a feature file has a line containing '@AUTH-001' that is NOT immediately followed by a 'Scenario:' line
    write_feature(
        tmp.path(),
        "auth.feature",
        "Feature: Auth\n\n  @AUTH-001\n  # just a comment, not a scenario\n  Given x\n",
    );

    // @step When I dispatch validate-spec-alignment with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the returned JSON has valid=false
    let parsed = parse(&result.data);
    assert_eq!(parsed["valid"], json!(false), "got:\n{}", result.data);

    // @step And the returned JSON has warnings equal to ['No scenarios for AUTH-001']
    assert_eq!(
        parsed["warnings"],
        json!(["No scenarios for AUTH-001"]),
        "got:\n{}",
        result.data
    );
}

#[test]
fn errors_when_work_unit_id_is_omitted() {
    // Scenario: Errors when workUnitId is omitted

    // @step Given spec/work-units.json contains AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &["AUTH-001"]);

    // @step When I dispatch validate-spec-alignment with no workUnitId
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false with an error message indicating the work unit id is required
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message").to_lowercase();
    assert!(
        msg.contains("work unit") && msg.contains("requir"),
        "error message must indicate the work unit id is required; got: {msg}"
    );
}

#[test]
fn errors_when_work_units_json_is_malformed() {
    // Scenario: Errors when work-units.json is malformed

    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    let spec = tmp.path().join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), "{ not json").expect("write malformed");

    // @step When I dispatch validate-spec-alignment with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=false with an error message containing 'Failed to validate spec alignment:'
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Failed to validate spec alignment:"),
        "error message missing substring: {msg}"
    );
}

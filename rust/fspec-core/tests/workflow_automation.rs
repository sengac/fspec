#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/workflow-automation-rust-port.feature
//
// Dispatcher-level acceptance tests for the Rust port of `workflow-automation`
// (RPC-326). Each scenario in workflow-automation-rust-port.feature maps to
// exactly one #[test] function with @step comments mirroring the Gherkin
// verbatim.
//
// PHASE B (red): until the stub at
// rust/fspec-core/src/commands/workflow_automation.rs is replaced AND the
// command is added to PORTED_COMMANDS, every dispatch returns NotYetPorted and
// these assertions fail. That is the expected red phase.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "workflow-automation".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn seed_work_units(project_root: &Path, value: Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("create spec dir");
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&value).expect("serialize seed"),
    )
    .expect("write work-units.json");
}

fn write_feature(project_root: &Path, name: &str, body: &str) {
    let features = project_root.join("spec").join("features");
    fs::create_dir_all(&features).expect("create features dir");
    fs::write(features.join(name), body).expect("write feature file");
}

fn read_stored(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("stored work-units.json is valid JSON")
}

fn read_stored_raw(project_root: &Path) -> String {
    fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json")
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

/// A single work unit `AUTH-001` in the given status, with the states index
/// listing it under that status array.
fn unit_in_status(status: &str) -> Value {
    let listed = |s: &str| -> Value {
        if s == status {
            json!(["AUTH-001"])
        } else {
            json!([])
        }
    };
    json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": {
            "AUTH-001": {
                "id": "AUTH-001", "title": "Login", "status": status,
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": [], "specifying": [],
            "testing": listed("testing"),
            "implementing": listed("implementing"),
            "validating": listed("validating"),
            "done": [], "blocked": []
        }
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: record-iteration increments the nested metrics counter
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_record_iteration_increments_nested_metrics_counter() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with no metrics
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_in_status("implementing"));

    // @step When I dispatch workflow-automation with action 'record-iteration' and workUnitId 'AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "action": "record-iteration", "workUnitId": "AUTH-001" }),
    ));

    // @step Then the dispatch result succeeds
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned JSON shows success true and iterations 1
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));
    assert_eq!(data["iterations"].as_u64(), Some(1));

    // @step And the persisted AUTH-001 has metrics.iterations equal to 1
    let stored = read_stored(tmp.path());
    assert_eq!(
        stored["workUnits"]["AUTH-001"]["metrics"]["iterations"].as_u64(),
        Some(1)
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: auto-advance action advances a testing unit and records state history
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_auto_advance_advances_testing_unit_and_records_state_history() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with status 'testing'
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_in_status("testing"));

    // @step When I dispatch workflow-automation with action 'auto-advance', workUnitId 'AUTH-001', event 'tests-pass', and fromState 'testing'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "action": "auto-advance", "workUnitId": "AUTH-001", "event": "tests-pass", "fromState": "testing" }),
    ));

    // @step Then the dispatch result succeeds
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned JSON shows success true and newState 'implementing'
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));
    assert_eq!(data["newState"].as_str(), Some("implementing"));

    // @step And the persisted AUTH-001 status is 'implementing'
    let stored = read_stored(tmp.path());
    assert_eq!(
        stored["workUnits"]["AUTH-001"]["status"].as_str(),
        Some("implementing")
    );

    // @step And the persisted AUTH-001 stateHistory contains an entry with state 'implementing'
    let history = stored["workUnits"]["AUTH-001"]["stateHistory"]
        .as_array()
        .expect("stateHistory array");
    assert!(
        history
            .iter()
            .any(|e| e["state"].as_str() == Some("implementing")),
        "stateHistory must contain an implementing entry; got {history:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: auto-advance action supports the specs-complete transition
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_auto_advance_supports_specs_complete_transition() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with status 'specifying'
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_in_status("specifying"));

    // @step When I dispatch workflow-automation with action 'auto-advance', workUnitId 'AUTH-001', event 'specs-complete', and fromState 'specifying'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "action": "auto-advance", "workUnitId": "AUTH-001", "event": "specs-complete", "fromState": "specifying" }),
    ));

    // @step Then the dispatch result succeeds
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned JSON shows success true and newState 'testing'
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));
    assert_eq!(data["newState"].as_str(), Some("testing"));

    // @step And the persisted AUTH-001 status is 'testing'
    let stored = read_stored(tmp.path());
    assert_eq!(
        stored["workUnits"]["AUTH-001"]["status"].as_str(),
        Some("testing")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: validate-alignment counts tagged scenarios without writing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_validate_alignment_counts_tagged_scenarios_without_writing() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_in_status("implementing"));

    // @step And two feature files under spec/features tagged @AUTH-001
    write_feature(
        tmp.path(),
        "login.feature",
        "@AUTH-001\nFeature: Login\n\n  Scenario: A\n    Given x\n",
    );
    write_feature(
        tmp.path(),
        "session.feature",
        "@AUTH-001\nFeature: Session\n\n  Scenario: B\n    Given y\n",
    );
    let before_raw = read_stored_raw(tmp.path());

    // @step When I dispatch workflow-automation with action 'validate-alignment' and workUnitId 'AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "action": "validate-alignment", "workUnitId": "AUTH-001" }),
    ));

    // @step Then the dispatch result succeeds
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned JSON shows aligned true and scenariosFound greater than 0
    let data = parse_data(&result.data);
    assert_eq!(data["aligned"].as_bool(), Some(true));
    assert!(
        data["scenariosFound"].as_u64().unwrap_or(0) > 0,
        "scenariosFound must be > 0; got {data:?}"
    );

    // @step And spec/work-units.json is left byte-for-byte unchanged
    let after_raw = read_stored_raw(tmp.path());
    assert_eq!(
        before_raw, after_raw,
        "validate-alignment must not modify work-units.json"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Unknown action is rejected
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_unknown_action_is_rejected() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_in_status("implementing"));

    // @step When I dispatch workflow-automation with action 'frobnicate' and workUnitId 'AUTH-001'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "action": "frobnicate", "workUnitId": "AUTH-001" }),
    ));

    // @step Then the dispatch result fails
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains 'Invalid action: frobnicate'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Invalid action: frobnicate"),
        "unexpected error: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: auto-advance action rejects a state mismatch
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_auto_advance_rejects_state_mismatch() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with status 'implementing'
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_in_status("implementing"));

    // @step When I dispatch workflow-automation with action 'auto-advance', workUnitId 'AUTH-001', event 'tests-pass', and fromState 'testing'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "action": "auto-advance", "workUnitId": "AUTH-001", "event": "tests-pass", "fromState": "testing" }),
    ));

    // @step Then the dispatch result fails
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains "Work unit 'AUTH-001' is in state 'implementing', expected 'testing'"
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit 'AUTH-001' is in state 'implementing', expected 'testing'"),
        "unexpected error: {err}"
    );
}

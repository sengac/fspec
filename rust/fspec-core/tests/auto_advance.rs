#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/auto-advance-rust-port.feature
//
// Dispatcher-level acceptance tests for the Rust port of `auto-advance`
// (RPC-198). Each scenario in auto-advance-rust-port.feature maps to exactly
// one #[test] function with @step comments mirroring the Gherkin verbatim.
//
// PHASE B (red): until the stub at
// rust/fspec-core/src/commands/auto_advance.rs is replaced AND the command
// is added to PORTED_COMMANDS, every dispatch returns NotYetPorted and these
// assertions fail. That is the expected red phase.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "auto-advance".to_string(),
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

fn read_stored(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("stored work-units.json is valid JSON")
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
// Scenario: Dispatcher advances a testing work unit to implementing on tests-pass
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_advances_testing_unit_to_implementing_on_tests_pass() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with status 'testing'
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_in_status("testing"));

    // @step When I dispatch auto-advance through fspec_core::dispatch::dispatch_command with workUnitId 'AUTH-001', from 'testing', and event 'tests-pass'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "from": "testing", "event": "tests-pass" }),
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

    // @step And AUTH-001 is removed from the states.testing array and present in the states.implementing array
    let testing_has_auth = stored["states"]["testing"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str() == Some("AUTH-001"));
    assert!(!testing_has_auth, "must be removed from testing");
    let implementing_has_auth = stored["states"]["implementing"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str() == Some("AUTH-001"));
    assert!(implementing_has_auth, "must be present in implementing");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher advances a validating work unit to done and records completion
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_advances_validating_unit_to_done_and_records_completion() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with status 'validating'
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_in_status("validating"));

    // @step When I dispatch auto-advance with workUnitId 'AUTH-001', from 'validating', and event 'validation-pass'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "from": "validating", "event": "validation-pass" }),
    ));

    // @step Then the dispatch result succeeds
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned JSON shows success true and newState 'done'
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));
    assert_eq!(data["newState"].as_str(), Some("done"));

    // @step And the persisted AUTH-001 status is 'done'
    let stored = read_stored(tmp.path());
    assert_eq!(
        stored["workUnits"]["AUTH-001"]["status"].as_str(),
        Some("done")
    );

    // @step And the persisted AUTH-001 has a non-empty completedAt timestamp
    let completed = stored["workUnits"]["AUTH-001"]["completedAt"].as_str();
    assert!(
        completed.is_some() && !completed.unwrap().is_empty(),
        "completedAt must be set; got {completed:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects an undefined transition
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_rejects_undefined_transition() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with status 'testing'
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_in_status("testing"));

    // @step When I dispatch auto-advance with workUnitId 'AUTH-001', from 'testing', and event 'bogus'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "from": "testing", "event": "bogus" }),
    ));

    // @step Then the dispatch result fails
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains 'No transition defined for testing + bogus'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("No transition defined for testing + bogus"),
        "unexpected error: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects a state mismatch
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_rejects_state_mismatch() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with status 'implementing'
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_in_status("implementing"));

    // @step When I dispatch auto-advance with workUnitId 'AUTH-001', from 'testing', and event 'tests-pass'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "from": "testing", "event": "tests-pass" }),
    ));

    // @step Then the dispatch result fails
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains 'Work unit is in implementing state, expected testing'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit is in implementing state, expected testing"),
        "unexpected error: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Dispatcher rejects a missing work unit with the wrapped prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_rejects_missing_work_unit() {
    // @step Given a project root whose spec/work-units.json contains no work unit MISSING-001
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_in_status("testing"));

    // @step When I dispatch auto-advance with workUnitId 'MISSING-001', from 'testing', and event 'tests-pass'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "MISSING-001", "from": "testing", "event": "tests-pass" }),
    ));

    // @step Then the dispatch result fails
    assert!(!result.success, "expected failure, got {result:?}");

    // @step And the error message contains 'Work unit MISSING-001 not found'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit MISSING-001 not found"),
        "unexpected error: {err}"
    );
}

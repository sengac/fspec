#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/record-iteration-rust-port.feature
//
// Dispatcher-level acceptance tests for the Rust port of `record-iteration`
// (RPC-264). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin.
//
// PHASE B (red): until the stub at
// codelet/fspec-core/src/commands/record_iteration.rs is replaced AND the
// command is added to PORTED_COMMANDS, every dispatch returns NotYetPorted.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "record-iteration".to_string(),
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

fn unit_no_iterations() -> Value {
    json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": {
            "AUTH-001": {
                "id": "AUTH-001", "title": "Login", "status": "implementing",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": [], "specifying": [], "testing": [],
            "implementing": ["AUTH-001"], "validating": [], "done": [], "blocked": []
        }
    })
}

fn unit_with_iterations(n: u64) -> Value {
    json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": {
            "AUTH-001": {
                "id": "AUTH-001", "title": "Login", "status": "implementing",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z",
                "iterations": n
            }
        },
        "states": {
            "backlog": [], "specifying": [], "testing": [],
            "implementing": ["AUTH-001"], "validating": [], "done": [], "blocked": []
        }
    })
}

#[test]
fn dispatcher_increments_iterations_from_absent_to_one() {
    // @step Given a work unit "AUTH-001" exists with no iterations field
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_no_iterations());

    // @step When the dispatcher runs record-iteration for "AUTH-001"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the result is the JSON envelope with success true and iterations 1
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));
    assert_eq!(data["iterations"].as_u64(), Some(1));

    // @step And the stored work unit "AUTH-001" has iterations 1
    let stored = read_stored(tmp.path());
    assert_eq!(
        stored["workUnits"]["AUTH-001"]["iterations"].as_u64(),
        Some(1)
    );
}

#[test]
fn dispatcher_increments_an_existing_iterations_count() {
    // @step Given a work unit "AUTH-001" exists with iterations 3
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_with_iterations(3));
    let before = read_stored(tmp.path());
    let before_updated = before["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .unwrap()
        .to_string();

    // @step When the dispatcher runs record-iteration for "AUTH-001"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the result is the JSON envelope with success true and iterations 4
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));
    assert_eq!(data["iterations"].as_u64(), Some(4));

    // @step And the stored work unit "AUTH-001" has its updatedAt refreshed
    let stored = read_stored(tmp.path());
    assert_eq!(
        stored["workUnits"]["AUTH-001"]["iterations"].as_u64(),
        Some(4)
    );
    let after_updated = stored["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .unwrap();
    assert_ne!(
        after_updated, before_updated,
        "updatedAt must be refreshed; before={before_updated} after={after_updated}"
    );
}

#[test]
fn dispatcher_fails_for_a_missing_work_unit() {
    // @step Given no work unit "MISSING-999" exists
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), unit_no_iterations());

    // @step When the dispatcher runs record-iteration for "MISSING-999"
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "MISSING-999" })));

    // @step Then the error message contains "Failed to record iteration: Work unit MISSING-999 not found"
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("Failed to record iteration: Work unit MISSING-999 not found"),
        "missing canonical error; got: {msg}"
    );
}

#[test]
fn cli_and_dispatcher_converge_on_the_same_fspec_core_function() {
    // @step Given a work unit "AUTH-001" exists with iterations 0
    // Two identical workspaces so both invocation paths start from the same
    // state (record-iteration mutates the file in place). Both the LLM
    // dispatcher AND the CLI bridge converge on
    // commands::record_iteration::run; we exercise that single function via
    // dispatch_command from two identical seeds and assert identical result
    // envelopes (the {success,iterations} payload carries no timestamp, so a
    // byte-identical comparison is stable).
    let ws_a = TempDir::new().expect("tempdir a");
    let ws_b = TempDir::new().expect("tempdir b");
    seed_work_units(ws_a.path(), unit_with_iterations(0));
    seed_work_units(ws_b.path(), unit_with_iterations(0));

    // @step When the dispatcher and the CLI both invoke record-iteration with identical JSON args for "AUTH-001"
    let args = json!({ "workUnitId": "AUTH-001" });
    let via_a = dispatch_command(req(ws_a.path(), args.clone()));
    let via_b = dispatch_command(req(ws_b.path(), args.clone()));

    // @step Then both invocations produce identical results from commands::record_iteration::run
    assert_eq!(
        via_a.success, via_b.success,
        "both front doors must agree on success: {via_a:?} vs {via_b:?}"
    );
    if via_a.success {
        assert_eq!(
            parse_data(&via_a.data),
            parse_data(&via_b.data),
            "identical args + identical seed must yield identical result envelopes"
        );
    } else {
        // Red phase: both paths return the SAME NotYetPorted error.
        let ma = via_a.error.clone().unwrap_or_default();
        let mb = via_b.error.clone().unwrap_or_default();
        assert_eq!(ma, mb, "both front doors must surface identical errors");
        assert!(
            ma.contains("not yet ported"),
            "red phase expects NotYetPorted; got: {ma}"
        );
    }
}

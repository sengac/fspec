#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/discover-event-storm-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `discover-event-storm`
// (RPC-225). Each scenario maps to one #[test] fn with @step comments
// mirroring the Gherkin steps verbatim.
//
// discover-event-storm is a READ-ONLY command: it validates the work unit is in
// `specifying` status, then emits the Event Storm guidance wrapped in a
// <system-reminder>. A missing spec/work-units.json is an ERROR (Option B —
// NOT auto-created), matching add-domain-event. Tests drive the LLM-facing
// dispatcher (fspec_core::dispatch_command); until Phase C wiring they fail
// with NotYetPorted, which is the intended red phase.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "discover-event-storm".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

/// Seed a work-units.json with the given (id, status) pairs.
fn seed_units(units: &[(&str, &str)]) -> Value {
    let mut wus = serde_json::Map::new();
    let mut states: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        states.insert(*st, Vec::new());
    }
    for (id, status) in units {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("type".into(), Value::String("story".to_string()));
        obj.insert("status".into(), Value::String((*status).to_string()));
        obj.insert(
            "createdAt".into(),
            Value::String("2026-06-01T00:00:00.000Z".to_string()),
        );
        obj.insert(
            "updatedAt".into(),
            Value::String("2026-06-01T00:00:00.000Z".to_string()),
        );
        wus.insert((*id).to_string(), Value::Object(obj));
        states
            .get_mut(*status)
            .expect("known state")
            .push((*id).to_string());
    }
    let mut states_obj = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        states_obj.insert(
            (*st).to_string(),
            Value::Array(
                states
                    .get(*st)
                    .expect("seeded state")
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": Value::Object(states_obj),
    })
}

fn write_value(project_root: &Path, v: &Value) {
    write_work_units(project_root, &serde_json::to_string_pretty(v).unwrap());
}

// ---------- scenarios ----------

#[test]
fn dispatcher_emits_guidance_for_a_work_unit_in_specifying_status() {
    // @step Given spec/work-units.json contains AUTH-001 in specifying status
    let tmp = TempDir::new().expect("tempdir");
    write_value(tmp.path(), &seed_units(&[("AUTH-001", "specifying")]));
    let pre_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();

    // @step When I dispatch discover-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the output contains the green line '✓ Event Storm discovery session started for AUTH-001'
    assert!(
        result
            .data
            .contains("✓ Event Storm discovery session started for AUTH-001"),
        "expected green confirmation line; got:\n{}",
        result.data
    );

    // @step Then the output contains a <system-reminder> block beginning with 'EVENT STORM DISCOVERY - AUTH-001'
    assert!(
        result.data.contains("<system-reminder>"),
        "expected a system-reminder block; got:\n{}",
        result.data
    );
    assert!(
        result.data.contains("EVENT STORM DISCOVERY - AUTH-001"),
        "expected reminder header; got:\n{}",
        result.data
    );

    // @step Then the output ends the reminder body with the hint 'When done, run: fspec generate-example-mapping-from-event-storm AUTH-001'
    assert!(
        result
            .data
            .contains("When done, run: fspec generate-example-mapping-from-event-storm AUTH-001"),
        "expected next-step hint; got:\n{}",
        result.data
    );

    // @step Then spec/work-units.json is byte-identical after the call (read-only command)
    let post_bytes = fs::read(tmp.path().join("spec/work-units.json")).unwrap();
    assert_eq!(
        pre_bytes, post_bytes,
        "discover-event-storm must be read-only and not mutate work-units.json"
    );
}

#[test]
fn dispatcher_returns_missing_file_error_without_auto_creating() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/work-units.json").exists());

    // @step When I dispatch discover-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success=false with an error message exactly 'spec/work-units.json not found. Run fspec init first.'
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("spec/work-units.json not found. Run fspec init first."),
        "expected TS-parity missing-file message; got: {err}"
    );

    // @step Then spec/work-units.json does NOT exist after the call
    assert!(
        !tmp.path().join("spec/work-units.json").exists(),
        "discover-event-storm must NOT auto-create the file (Option B)"
    );
}

#[test]
fn dispatcher_returns_work_unit_not_found_when_id_absent() {
    // @step Given spec/work-units.json contains BUG-001 but not AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    write_value(tmp.path(), &seed_units(&[("BUG-001", "specifying")]));

    // @step When I dispatch discover-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success=false with an error message exactly 'Work unit AUTH-001 not found'
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit AUTH-001 not found"),
        "expected missing work-unit message; got: {err}"
    );
}

#[test]
fn dispatcher_rejects_a_work_unit_not_in_specifying_status() {
    // @step Given spec/work-units.json contains AUTH-001 in backlog status
    let tmp = TempDir::new().expect("tempdir");
    write_value(tmp.path(), &seed_units(&[("AUTH-001", "backlog")]));

    // @step When I dispatch discover-event-storm with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({"workUnitId": "AUTH-001"})));

    // @step Then the dispatcher returns success=false with an error message containing 'Work unit AUTH-001 must be in specifying status (currently: backlog)'
    assert!(!result.success, "expected failure; got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Work unit AUTH-001 must be in specifying status (currently: backlog)"),
        "expected status-gate message; got: {err}"
    );

    // @step Then the error message also contains 'Run: fspec update-work-unit-status AUTH-001 specifying'
    assert!(
        err.contains("Run: fspec update-work-unit-status AUTH-001 specifying"),
        "expected update-work-unit-status hint; got: {err}"
    );
}

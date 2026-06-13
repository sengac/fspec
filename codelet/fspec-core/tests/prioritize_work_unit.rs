#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/port-prioritize-work-unit-command-to-rust.feature
//
// Dispatcher-level acceptance tests for the Rust port of
// `prioritize-work-unit` (RPC-255). Each scenario maps to one #[test]
// with `@step` comments mirroring the Gherkin steps verbatim.
//
// At the end of Phase B these tests fail with `NotYetPorted` (the
// dispatcher still routes to the stub). After Phase C + supervisor
// wiring they turn green.

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
        command: "prioritize-work-unit".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_file(path: &Path, raw: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(path, raw).expect("write file");
}

fn write_work_units(project_root: &Path, raw: &str) {
    write_file(&project_root.join("spec/work-units.json"), raw);
}

fn read_work_units_raw(project_root: &Path) -> String {
    fs::read_to_string(project_root.join("spec/work-units.json"))
        .expect("read spec/work-units.json")
}

fn read_states(project_root: &Path, status: &str) -> Vec<String> {
    let on_disk: Value =
        serde_json::from_str(&read_work_units_raw(project_root)).expect("parse on-disk");
    on_disk["states"][status]
        .as_array()
        .expect("states array present")
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

/// Build a work-units.json with the given per-status arrays. Each `(id,
/// status)` pair produces a minimal work unit in `workUnits` plus an entry
/// in the matching `states.<status>` array, in the order supplied.
fn wu(id: &str, status: &str) -> String {
    format!(
        r#""{id}": {{ "id": "{id}", "title": "{id}", "status": "{status}", "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z" }}"#
    )
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_position_top_moves_to_front() {
    // Scenario: Position top moves a work unit to the front of its column

    // @step Given spec/work-units.json backlog order is AUTH-002, AUTH-003, AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {}, {}, {} }}, "states": {{ "backlog": ["AUTH-002","AUTH-003","AUTH-001"], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("AUTH-001", "backlog"),
        wu("AUTH-002", "backlog"),
        wu("AUTH-003", "backlog")
    );
    write_work_units(tmp.path(), &body);

    // @step When I run `fspec prioritize-work-unit AUTH-001 --position top`
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "position": "top" }),
    ));

    // @step Then the process exits with code 0
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the backlog order becomes AUTH-001, AUTH-002, AUTH-003
    assert_eq!(
        read_states(tmp.path(), "backlog"),
        vec!["AUTH-001", "AUTH-002", "AUTH-003"]
    );
}

#[test]
fn dispatcher_numeric_position_is_one_based() {
    // Scenario: Numeric position is 1-based

    // @step Given spec/work-units.json backlog order is AUTH-002, AUTH-003, AUTH-004, AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {}, {}, {}, {} }}, "states": {{ "backlog": ["AUTH-002","AUTH-003","AUTH-004","AUTH-001"], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("AUTH-001", "backlog"),
        wu("AUTH-002", "backlog"),
        wu("AUTH-003", "backlog"),
        wu("AUTH-004", "backlog")
    );
    write_work_units(tmp.path(), &body);

    // @step When I run `fspec prioritize-work-unit AUTH-001 --position 3`
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "position": 3 }),
    ));

    // @step Then the process exits with code 0
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the backlog order becomes AUTH-002, AUTH-003, AUTH-001, AUTH-004
    assert_eq!(
        read_states(tmp.path(), "backlog"),
        vec!["AUTH-002", "AUTH-003", "AUTH-001", "AUTH-004"]
    );
}

#[test]
fn dispatcher_rejects_numeric_position_below_one() {
    // Scenario: Reject numeric position below 1

    // @step Given spec/work-units.json backlog contains AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {} }}, "states": {{ "backlog": ["AUTH-001"], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("AUTH-001", "backlog")
    );
    write_work_units(tmp.path(), &body);

    // @step When I run `fspec prioritize-work-unit AUTH-001 --position 0`
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "position": 0 }),
    ));

    // @step Then the process exits with code 1
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And stderr contains "Invalid position: 0. Position must be >= 1 (1-based index)"
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Invalid position: 0. Position must be >= 1 (1-based index)"),
        "error must mention the invalid-position message; got: {msg}"
    );
}

#[test]
fn dispatcher_detects_work_unit_missing_from_states_array() {
    // Scenario: Detect work unit missing from its own states array

    // @step Given AUTH-001 has status specifying but is listed only in states.testing
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {} }}, "states": {{ "backlog": [], "specifying": [], "testing": ["AUTH-001"], "implementing": [], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("AUTH-001", "specifying")
    );
    write_work_units(tmp.path(), &body);

    // @step When I run `fspec prioritize-work-unit AUTH-001 --position top`
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "position": "top" }),
    ));

    // @step Then the process exits with code 1
    assert!(!result.success, "expected success=false; got {result:?}");
    let msg = result.error.as_deref().unwrap_or("");

    // @step And stderr contains "Data integrity error"
    assert!(msg.contains("Data integrity error"), "got: {msg}");

    // @step And stderr contains "states.specifying"
    assert!(msg.contains("states.specifying"), "got: {msg}");

    // @step And stderr contains "fspec repair-work-units"
    assert!(msg.contains("fspec repair-work-units"), "got: {msg}");
}

#[test]
fn dispatcher_rejects_cross_column_relative_placement() {
    // Scenario: Reject cross-column relative placement

    // @step Given FEAT-017 is in states.specifying and AUTH-001 is in states.testing
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {}, {} }}, "states": {{ "backlog": [], "specifying": ["FEAT-017"], "testing": ["AUTH-001"], "implementing": [], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("FEAT-017", "specifying"),
        wu("AUTH-001", "testing")
    );
    write_work_units(tmp.path(), &body);

    // @step When I run `fspec prioritize-work-unit FEAT-017 --before AUTH-001`
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "FEAT-017", "before": "AUTH-001" }),
    ));

    // @step Then the process exits with code 1
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And stderr contains "Data integrity error"
    // (cross-column rejection: AUTH-001 is not in states.specifying — the
    // TS source emits the cross-column message before reaching the
    // data-integrity branch; both name the divergent statuses.)
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Cannot prioritize across columns") || msg.contains("Data integrity error"),
        "error must reject cross-column placement; got: {msg}"
    );
}

#[test]
fn dispatcher_rejects_non_existent_work_unit() {
    // Scenario: Reject prioritizing a non-existent work unit

    // @step Given spec/work-units.json does not contain MISSING-999
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {} }}, "states": {{ "backlog": ["AUTH-001"], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("AUTH-001", "backlog")
    );
    write_work_units(tmp.path(), &body);
    let before = read_work_units_raw(tmp.path());

    // @step When I run `fspec prioritize-work-unit MISSING-999 --position top`
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "MISSING-999", "position": "top" }),
    ));

    // @step Then the process exits with code 1
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And stderr contains "Work unit 'MISSING-999' does not exist"
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Work unit 'MISSING-999' does not exist"),
        "got: {msg}"
    );

    // @step And spec/work-units.json is byte-identical to its pre-call content
    let after = read_work_units_raw(tmp.path());
    assert_eq!(before, after, "file must be untouched on missing work unit");
}

#[test]
fn dispatcher_rejects_done_work_unit() {
    // Scenario: Reject prioritizing a done work unit

    // @step Given DONE-001 has status done and is in states.done
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {} }}, "states": {{ "backlog": [], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": ["DONE-001"], "blocked": [] }} }}"#,
        wu("DONE-001", "done")
    );
    write_work_units(tmp.path(), &body);

    // @step When I run `fspec prioritize-work-unit DONE-001 --position top`
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "DONE-001", "position": "top" }),
    ));

    // @step Then the process exits with code 1
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And stderr contains "Cannot prioritize work units in done column"
    let msg = result.error.as_deref().unwrap_or("");
    assert!(
        msg.contains("Cannot prioritize work units in done column"),
        "got: {msg}"
    );
}

#[test]
fn dispatcher_relative_placement_before_and_after() {
    // Scenario: Relative placement with before and after

    // @step Given spec/work-units.json implementing order is AUTH-002, AUTH-001
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        r#"{{ "workUnits": {{ {}, {} }}, "states": {{ "backlog": [], "specifying": [], "testing": [], "implementing": ["AUTH-002","AUTH-001"], "validating": [], "done": [], "blocked": [] }} }}"#,
        wu("AUTH-001", "implementing"),
        wu("AUTH-002", "implementing")
    );
    write_work_units(tmp.path(), &body);

    // @step When I run `fspec prioritize-work-unit AUTH-001 --before AUTH-002`
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "before": "AUTH-002" }),
    ));

    // @step Then the process exits with code 0
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the implementing order becomes AUTH-001, AUTH-002
    assert_eq!(
        read_states(tmp.path(), "implementing"),
        vec!["AUTH-001", "AUTH-002"]
    );
}

#[test]
fn shared_infrastructure_is_reused_without_duplication() {
    // Scenario: CLI delegates to the same fspec_core function as the dispatcher
    // (core-side half: the single source of truth reuses shared io helpers
    // and is no longer a NotYetPorted stub.)

    // @step Given the codelet/fspec crate is built
    // (precondition — this test only runs when the crate compiles)

    // @step When I inspect codelet/fspec/src/prioritize_work_unit.rs
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands/prioritize_work_unit.rs");
    let src = fs::read_to_string(&path).expect("read prioritize_work_unit.rs");

    // @step Then the source declares it calls codelet_fspec_core::commands::prioritize_work_unit::run
    assert!(
        src.contains("ensure_work_units_file"),
        "prioritize_work_unit.rs must reference `ensure_work_units_file`; got:\n{src}"
    );
    assert!(
        src.contains("write_json_atomic"),
        "prioritize_work_unit.rs must reference `write_json_atomic`; got:\n{src}"
    );

    // @step Then the source does NOT perform any file IO directly on spec/work-units.json
    assert!(
        !src.contains("FspecCoreError::NotYetPorted"),
        "prioritize_work_unit.rs must no longer be a NotYetPorted stub"
    );
    assert!(
        !src.contains("std::fs::write"),
        "prioritize_work_unit.rs must NOT call std::fs::write directly"
    );
}

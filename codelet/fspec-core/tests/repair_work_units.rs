#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/port-repair-work-units-command-to-rust.feature
//
// Dispatcher-level acceptance tests for the Rust port of
// `repair-work-units` (RPC-284). Each scenario maps to one #[test]
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
        command: "repair-work-units".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
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

fn on_disk(project_root: &Path) -> Value {
    let raw = fs::read_to_string(project_root.join("spec/work-units.json"))
        .expect("read spec/work-units.json");
    serde_json::from_str(&raw).expect("parse on-disk")
}

fn repairs_contains(result_data: &Value, needle: &str) -> bool {
    result_data["repairs"]
        .as_array()
        .map(|arr| arr.iter().any(|v| v.as_str() == Some(needle)))
        .unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn dispatcher_moves_work_unit_into_matching_states_array() {
    // Scenario: Move a work unit into the states array matching its status

    // @step Given AUTH-001 has status specifying but is listed only in states.testing
    let tmp = TempDir::new().expect("tempdir");
    let body = r#"{
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "Auth", "status": "specifying", "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z" }
  },
  "states": { "backlog": [], "specifying": [], "testing": ["AUTH-001"], "implementing": [], "validating": [], "done": [], "blocked": [] }
}"#;
    write_work_units(tmp.path(), body);

    // @step When I dispatch repair-work-units
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "expected success=true; got {result:?}");
    let data = parse_data(&result.data);

    // @step Then states.specifying contains AUTH-001 and states.testing does not
    let disk = on_disk(tmp.path());
    let specifying_has = disk["states"]["specifying"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str() == Some("AUTH-001"));
    let testing_has = disk["states"]["testing"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str() == Some("AUTH-001"));
    assert!(specifying_has, "specifying must contain AUTH-001");
    assert!(!testing_has, "testing must not contain AUTH-001");

    // @step And the repairs array contains "Moved AUTH-001 from testing to specifying"
    assert!(
        repairs_contains(&data, "Moved AUTH-001 from testing to specifying"),
        "repairs must contain the move message; got: {data:?}"
    );
}

#[test]
fn dispatcher_repairs_missing_blocked_by_reverse_link() {
    // Scenario: Repair a missing blockedBy reverse link

    // @step Given AUTH-001 has blocks AUTH-002 but AUTH-002 has no blockedBy entry
    let tmp = TempDir::new().expect("tempdir");
    let body = r#"{
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "A1", "status": "backlog", "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z", "blocks": ["AUTH-002"] },
    "AUTH-002": { "id": "AUTH-002", "title": "A2", "status": "backlog", "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z" }
  },
  "states": { "backlog": ["AUTH-001","AUTH-002"], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] }
}"#;
    write_work_units(tmp.path(), body);

    // @step When I dispatch repair-work-units
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "expected success=true; got {result:?}");
    let data = parse_data(&result.data);

    // @step Then AUTH-002.blockedBy contains AUTH-001
    let disk = on_disk(tmp.path());
    let blocked_by_has = disk["workUnits"]["AUTH-002"]["blockedBy"]
        .as_array()
        .expect("AUTH-002.blockedBy present")
        .iter()
        .any(|v| v.as_str() == Some("AUTH-001"));
    assert!(blocked_by_has, "AUTH-002.blockedBy must contain AUTH-001");

    // @step And the repairs array contains "Repaired bidirectional link: AUTH-001 blocks AUTH-002"
    assert!(
        repairs_contains(
            &data,
            "Repaired bidirectional link: AUTH-001 blocks AUTH-002"
        ),
        "repairs must contain the blocks message; got: {data:?}"
    );
}

#[test]
fn dispatcher_repairs_missing_relates_to_reverse_link() {
    // Scenario: Repair a missing relatesTo reverse link

    // @step Given AUTH-001 has relatesTo AUTH-002 but AUTH-002 has no reverse relatesTo entry
    let tmp = TempDir::new().expect("tempdir");
    let body = r#"{
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "A1", "status": "backlog", "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z", "relatesTo": ["AUTH-002"] },
    "AUTH-002": { "id": "AUTH-002", "title": "A2", "status": "backlog", "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z" }
  },
  "states": { "backlog": ["AUTH-001","AUTH-002"], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] }
}"#;
    write_work_units(tmp.path(), body);

    // @step When I dispatch repair-work-units
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "expected success=true; got {result:?}");
    let data = parse_data(&result.data);

    // @step Then AUTH-002.relatesTo contains AUTH-001
    let disk = on_disk(tmp.path());
    let relates_to_has = disk["workUnits"]["AUTH-002"]["relatesTo"]
        .as_array()
        .expect("AUTH-002.relatesTo present")
        .iter()
        .any(|v| v.as_str() == Some("AUTH-001"));
    assert!(relates_to_has, "AUTH-002.relatesTo must contain AUTH-001");

    // @step And the repairs array contains "Repaired bidirectional link: AUTH-001 relates to AUTH-002"
    assert!(
        repairs_contains(
            &data,
            "Repaired bidirectional link: AUTH-001 relates to AUTH-002"
        ),
        "repairs must contain the relates-to message; got: {data:?}"
    );
}

#[test]
fn dispatcher_consistent_data_yields_zero_repairs() {
    // Scenario: Fully consistent data yields zero repairs

    // @step Given spec/work-units.json is fully consistent
    let tmp = TempDir::new().expect("tempdir");
    let body = r#"{
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "A1", "status": "backlog", "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z" }
  },
  "states": { "backlog": ["AUTH-001"], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] }
}"#;
    write_work_units(tmp.path(), body);

    // @step When I dispatch repair-work-units
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "expected success=true; got {result:?}");
    let data = parse_data(&result.data);

    // @step Then the result reports repaired 0 with an empty repairs array
    assert_eq!(
        data["repaired"].as_u64(),
        Some(0),
        "repaired must be 0; got {data:?}"
    );
    assert_eq!(
        data["repairs"].as_array().map(Vec::len),
        Some(0),
        "repairs array must be empty; got {data:?}"
    );
}

#[test]
fn dispatcher_returns_canonical_success_shape() {
    // Scenario: Fully consistent data yields zero repairs (success shape twin)
    // Exercises the { success, repairs, repaired } JSON contract from rule [5].

    // @step Given spec/work-units.json is fully consistent
    let tmp = TempDir::new().expect("tempdir");
    let body = r#"{
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "A1", "status": "backlog", "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z" }
  },
  "states": { "backlog": ["AUTH-001"], "specifying": [], "testing": [], "implementing": [], "validating": [], "done": [], "blocked": [] }
}"#;
    write_work_units(tmp.path(), body);

    // @step When I dispatch repair-work-units
    let result = dispatch_command(req(tmp.path(), json!({})));
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the result has fields success=true, repairs (array) and repaired (number)
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));
    assert!(data["repairs"].is_array(), "repairs must be an array");
    assert!(data["repaired"].is_u64(), "repaired must be a number");
}

#[test]
fn shared_infrastructure_is_reused_without_duplication() {
    // Scenario: CLI delegates to the same fspec_core function as the dispatcher
    // (core-side half — the single source of truth is no longer a stub.)

    // @step Given the codelet/fspec-core crate is built
    // (precondition — this test only runs when the crate compiles)

    // @step When I inspect codelet/fspec-core/src/commands/repair_work_units.rs
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands/repair_work_units.rs");
    let src = fs::read_to_string(&path).expect("read repair_work_units.rs");

    // @step Then the source references the shared io helpers
    assert!(
        src.contains("ensure_work_units_file"),
        "repair_work_units.rs must reference `ensure_work_units_file`; got:\n{src}"
    );
    assert!(
        src.contains("write_json_atomic"),
        "repair_work_units.rs must reference `write_json_atomic`; got:\n{src}"
    );

    // @step Then the source is no longer a NotYetPorted stub
    assert!(
        !src.contains("FspecCoreError::NotYetPorted"),
        "repair_work_units.rs must no longer be a NotYetPorted stub"
    );
}

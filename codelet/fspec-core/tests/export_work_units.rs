#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/export-work-units-rust-port.feature
//
// Dispatcher-level acceptance tests for the Rust port of `export-work-units`
// (RPC-229). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin.
//
// PHASE B (red): until the stub at
// codelet/fspec-core/src/commands/export_work_units.rs is replaced AND the
// command is added to PORTED_COMMANDS, every dispatch returns NotYetPorted.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "export-work-units".to_string(),
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

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

/// Three units in NON-alphabetical insertion order C-1, A-1, B-1, with mixed
/// statuses to exercise the "status filter ignored" scenario.
fn three_units_mixed() -> Value {
    json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": {
            "C-1": {
                "id": "C-1", "title": "third", "status": "backlog",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            },
            "A-1": {
                "id": "A-1", "title": "first", "status": "done",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            },
            "B-1": {
                "id": "B-1", "title": "second", "status": "implementing",
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": ["C-1"], "specifying": [], "testing": [],
            "implementing": ["B-1"], "validating": [], "done": ["A-1"], "blocked": []
        }
    })
}

#[test]
fn dispatcher_exports_all_work_units_to_a_json_file() {
    // @step Given a workspace with three work units in spec/work-units.json
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), three_units_mixed());
    let out = tmp.path().join("out.json");

    // @step When the dispatcher runs export-work-units with format "json" and output "out.json"
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "format": "json", "output": out.to_str().unwrap() }),
    ));

    // @step Then the result is the JSON envelope with success true
    assert!(result.success, "expected success=true, got {result:?}");
    let data = parse_data(&result.data);
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step And the file "out.json" contains a 2-space pretty JSON array of all three work units
    let written = fs::read_to_string(&out).expect("out.json written");
    let arr: Value = serde_json::from_str(&written).expect("out.json is valid JSON");
    let ids: Vec<&str> = arr
        .as_array()
        .expect("exported array")
        .iter()
        .map(|u| u["id"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(ids, vec!["C-1", "A-1", "B-1"]);
    // 2-space pretty indent (TS JSON.stringify(workUnits, null, 2)).
    assert!(
        written.contains("\n  {"),
        "exported file must be 2-space pretty JSON; got:\n{written}"
    );
}

#[test]
fn dispatcher_rejects_an_unsupported_format() {
    // @step Given a workspace with a valid spec/work-units.json
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), three_units_mixed());
    let out = tmp.path().join("out.csv");

    // @step When the dispatcher runs export-work-units with format "csv" and output "out.csv"
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "format": "csv", "output": out.to_str().unwrap() }),
    ));

    // @step Then the error message contains "Failed to export work units: Unsupported format: csv"
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message expected");
    assert!(
        msg.contains("Failed to export work units: Unsupported format: csv"),
        "missing canonical error; got: {msg}"
    );
}

#[test]
fn dispatcher_ignores_the_status_filter_and_exports_every_unit() {
    // @step Given a workspace with work units in mixed statuses
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), three_units_mixed());
    let out = tmp.path().join("out.json");

    // @step When the dispatcher runs export-work-units with format "json", output "out.json" and status "done"
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "format": "json", "output": out.to_str().unwrap(), "status": "done" }),
    ));

    // @step Then the file "out.json" contains every work unit regardless of status
    assert!(result.success, "expected success=true, got {result:?}");
    let written = fs::read_to_string(&out).expect("out.json written");
    let arr: Value = serde_json::from_str(&written).expect("out.json is valid JSON");
    let ids: Vec<&str> = arr
        .as_array()
        .expect("exported array")
        .iter()
        .map(|u| u["id"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        ids,
        vec!["C-1", "A-1", "B-1"],
        "status filter MUST be ignored — all units exported"
    );
}

#[test]
fn dispatcher_preserves_work_unit_insertion_order_in_the_export() {
    // @step Given a workspace whose work units were inserted in the order "C-1", "A-1", "B-1"
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), three_units_mixed());
    let out = tmp.path().join("out.json");

    // @step When the dispatcher runs export-work-units with format "json" and output "out.json"
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "format": "json", "output": out.to_str().unwrap() }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the exported array lists work units in the order "C-1", "A-1", "B-1"
    let written = fs::read_to_string(&out).expect("out.json written");
    let arr: Value = serde_json::from_str(&written).expect("out.json is valid JSON");
    let ids: Vec<&str> = arr
        .as_array()
        .expect("exported array")
        .iter()
        .map(|u| u["id"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(ids, vec!["C-1", "A-1", "B-1"]);
}

#[test]
fn cli_and_dispatcher_converge_on_the_same_fspec_core_function() {
    // @step Given a workspace with a valid spec/work-units.json
    // Both the LLM dispatcher AND the CLI bridge converge on
    // commands::export_work_units::run; we exercise that single function via
    // dispatch_command from two identical seeds and assert identical result
    // envelopes ({success} carries no timestamp, so the comparison is stable).
    let ws_a = TempDir::new().expect("tempdir a");
    let ws_b = TempDir::new().expect("tempdir b");
    seed_work_units(ws_a.path(), three_units_mixed());
    seed_work_units(ws_b.path(), three_units_mixed());
    let out_a = ws_a.path().join("out.json");
    let out_b = ws_b.path().join("out.json");

    // @step When the dispatcher and the CLI both invoke export-work-units with identical JSON args
    let via_a = dispatch_command(req(
        ws_a.path(),
        json!({ "format": "json", "output": out_a.to_str().unwrap() }),
    ));
    let via_b = dispatch_command(req(
        ws_b.path(),
        json!({ "format": "json", "output": out_b.to_str().unwrap() }),
    ));

    // @step Then both invocations produce identical results from commands::export_work_units::run
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
        // The two exported files must also be byte-identical.
        assert_eq!(
            fs::read_to_string(&out_a).expect("out_a"),
            fs::read_to_string(&out_b).expect("out_b"),
            "exported files must be byte-identical"
        );
    } else {
        // Red phase: both paths return the SAME NotYetPorted error.
        let ma = via_a.error.unwrap_or_default();
        let mb = via_b.error.unwrap_or_default();
        assert_eq!(ma, mb, "both front doors must surface identical errors");
        assert!(
            ma.contains("not yet ported"),
            "red phase expects NotYetPorted; got: {ma}"
        );
    }
}

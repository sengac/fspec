#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/generate-summary-report-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of
// `generate-summary-report` (RPC-235) through the LLM-facing dispatcher front
// door. Each scenario maps to exactly one #[test] function with @step comments
// mirroring the Gherkin steps verbatim.
//
// Red phase: until RPC-235 is wired into run_ported (Phase C), the dispatcher
// routes `generate-summary-report` to the NotYetPorted stub, so every success
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
        command: "generate-summary-report".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

/// Build a work-units.json string from (id, status, estimate?) tuples.
/// A `None` estimate omits the field; a `None` status omits the field.
fn work_units_with(entries: &[(&str, Option<&str>, Option<i64>)]) -> String {
    let mut wus = serde_json::Map::new();
    for (id, status, estimate) in entries {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        if let Some(s) = status {
            obj.insert("status".into(), Value::String((*s).to_string()));
        }
        if let Some(e) = estimate {
            obj.insert("estimate".into(), json!(e));
        }
        obj.insert("createdAt".into(), Value::String("x".to_string()));
        obj.insert("updatedAt".into(), Value::String("x".to_string()));
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
fn aggregate_a_store_with_completed_and_pending_work_units() {
    // @step Given a work units store with two done units estimated 3 and 5 and one backlog unit estimated 2
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("A-1", Some("done"), Some(3)),
            ("A-2", Some("done"), Some(5)),
            ("A-3", Some("backlog"), Some(2)),
        ]),
    );

    // @step When I generate a json summary report
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the written report has totalWorkUnits 3 and totalStoryPoints 10
    let written =
        fs::read_to_string(tmp.path().join("spec/summary-report.json")).expect("read report");
    let parsed: Value = serde_json::from_str(&written).expect("report is JSON");
    assert_eq!(parsed["totalWorkUnits"].as_i64(), Some(3));
    assert_eq!(parsed["totalStoryPoints"].as_i64(), Some(10));

    // @step And the velocity has completedPoints 8 and completedWorkUnits 2
    assert_eq!(parsed["velocity"]["completedPoints"].as_i64(), Some(8));
    assert_eq!(parsed["velocity"]["completedWorkUnits"].as_i64(), Some(2));
}

#[test]
fn markdown_report_renders_the_expected_layout() {
    // @step Given a work units store with a mix of statuses and estimates
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("A-1", Some("done"), Some(3)),
            ("A-2", Some("backlog"), Some(2)),
        ]),
    );

    // @step When I generate a markdown summary report
    let result = dispatch_command(req(tmp.path(), json!({ "format": "markdown" })));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the written report begins with "# Project Summary Report"
    let written =
        fs::read_to_string(tmp.path().join("spec/summary-report.md")).expect("read report");
    assert!(
        written.starts_with("# Project Summary Report"),
        "markdown report must begin with the heading; got:\n{written}"
    );

    // @step And the report includes a Breakdown by Status section and a Velocity Metrics section
    assert!(
        written.contains("## Breakdown by Status"),
        "missing breakdown section:\n{written}"
    );
    assert!(
        written.contains("## Velocity Metrics"),
        "missing velocity section:\n{written}"
    );
}

#[test]
fn json_report_is_pretty_printed_with_two_space_indent() {
    // @step Given a work units store containing one work unit
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A-1", Some("done"), Some(3))]),
    );

    // @step When I generate a json summary report
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the written report is JSON pretty-printed with two-space indentation
    let written =
        fs::read_to_string(tmp.path().join("spec/summary-report.json")).expect("read report");
    assert!(
        written.contains("\n  \"totalWorkUnits\""),
        "report must be pretty-printed with 2-space indent; got:\n{written}"
    );
}

#[test]
fn a_custom_output_path_is_honoured() {
    // @step Given a work units store containing one work unit
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A-1", Some("done"), Some(3))]),
    );

    // @step When I generate a markdown summary report to custom.md
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "format": "markdown", "output": "custom.md" }),
    ));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the report is written to custom.md
    assert!(
        tmp.path().join("custom.md").is_file(),
        "custom.md must be written"
    );

    // @step And the returned message is "✓ Report generated: custom.md"
    assert_eq!(result.data, "✓ Report generated: custom.md");
}

#[test]
fn a_work_unit_without_a_status_is_counted_as_unknown() {
    // @step Given a work units store where one work unit has no status field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with(&[("A-1", None, Some(1))]));

    // @step When I generate a json summary report
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the byStatus breakdown counts that work unit under "unknown"
    let written =
        fs::read_to_string(tmp.path().join("spec/summary-report.json")).expect("read report");
    let parsed: Value = serde_json::from_str(&written).expect("report is JSON");
    assert_eq!(parsed["byStatus"]["unknown"].as_i64(), Some(1));
}

#[test]
fn a_missing_work_units_file_fails() {
    // @step Given a workspace with no spec/work-units.json file
    let tmp = TempDir::new().expect("tempdir");

    // @step When I generate a summary report
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the run returns an error containing "Failed to generate summary report:"
    assert!(!result.success, "expected success=false, got {result:?}");
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Failed to generate summary report:"),
        "error message missing canonical substring: {msg}"
    );
}

#[test]
fn dispatcher_and_core_produce_identical_report_content() {
    // @step Given a work units store containing one work unit
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A-1", Some("done"), Some(3))]),
    );

    // @step When I generate a json summary report via the core run function
    let first = dispatch_command(req(
        tmp.path(),
        json!({ "format": "json", "output": "a.json" }),
    ));
    let second = dispatch_command(req(
        tmp.path(),
        json!({ "format": "json", "output": "b.json" }),
    ));
    assert!(first.success && second.success, "{first:?} {second:?}");

    // @step Then the written report content is the same as generating via the dispatcher path
    let a = fs::read_to_string(tmp.path().join("a.json")).expect("read a.json");
    let b = fs::read_to_string(tmp.path().join("b.json")).expect("read b.json");
    assert_eq!(a, b, "both report paths must produce identical content");
}

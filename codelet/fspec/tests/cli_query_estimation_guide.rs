//! CLI surface for the `query-estimation-guide` subcommand on the standalone
//! fspec Rust binary — RPC-259.
//!
//! Feature: spec/features/query-estimation-guide-cli-subcommand.feature
//! Feature: spec/features/query-estimation-guide-rust-port.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

mod common;

use common::fspec_bin;

fn run_query(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("query-estimation-guide");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec query-estimation-guide");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "query-estimation-guide".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

/// Build a work-units.json. `entries` is a slice of (id, status, extra_fields).
fn work_units_with(entries: &[(&str, &str, Value)]) -> String {
    let mut wus = serde_json::Map::new();
    for (id, status, extra) in entries {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("status".into(), Value::String((*status).to_string()));
        obj.insert("createdAt".into(), Value::String("x".to_string()));
        obj.insert("updatedAt".into(), Value::String("x".to_string()));
        if let Value::Object(map) = extra {
            for (k, v) in map {
                obj.insert(k.clone(), v.clone());
            }
        }
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
// rust-port scenarios (dispatcher contract)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_dispatcher_empty_workspace_returns_empty_patterns() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step And the returned JSON has patterns=[]
    let data = parse_data(&result.data);
    assert_eq!(data["patterns"].as_array().map(|a| a.len()), Some(0));

    // @step And spec/work-units.json exists after the call (auto-created by ensure_work_units_file)
    assert!(tmp.path().join("spec/work-units.json").exists());
}

#[test]
fn scenario_dispatcher_ignores_non_done_units() {
    // @step Given spec/work-units.json contains A with status='backlog', estimate=3, iterations=1
    // @step And spec/work-units.json also contains B with status='implementing', estimate=5, iterations=2
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("A", "backlog", json!({ "estimate": 3, "iterations": 1 })),
            ("B", "implementing", json!({ "estimate": 5, "iterations": 2 })),
        ]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has patterns=[]
    assert_eq!(data["patterns"].as_array().map(|a| a.len()), Some(0));
}

#[test]
fn scenario_dispatcher_skips_done_unit_missing_iterations() {
    // @step Given spec/work-units.json contains A with status='done', estimate=3 and no iterations field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "done", json!({ "estimate": 3 }))]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has patterns=[]
    assert_eq!(data["patterns"].as_array().map(|a| a.len()), Some(0));
}

#[test]
fn scenario_dispatcher_skips_done_unit_missing_estimate() {
    // @step Given spec/work-units.json contains A with status='done', iterations=1 and no estimate field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "done", json!({ "iterations": 1 }))]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has patterns=[]
    assert_eq!(data["patterns"].as_array().map(|a| a.len()), Some(0));
}

#[test]
fn scenario_dispatcher_single_done_unit_yields_low_confidence() {
    // @step Given spec/work-units.json contains A with status='done', estimate=3, iterations=1
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "done", json!({ "estimate": 3, "iterations": 1 }))]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has patterns containing exactly one entry {points=3, expectedIterations='1-1', confidence='low'}
    let arr = data["patterns"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["points"].as_u64(), Some(3));
    assert_eq!(arr[0]["expectedIterations"].as_str(), Some("1-1"));
    assert_eq!(arr[0]["confidence"].as_str(), Some("low"));
}

#[test]
fn scenario_dispatcher_two_done_units_yield_medium_confidence() {
    // @step Given spec/work-units.json contains A with status='done', estimate=3, iterations=1 and B with status='done', estimate=3, iterations=2
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("A", "done", json!({ "estimate": 3, "iterations": 1 })),
            ("B", "done", json!({ "estimate": 3, "iterations": 2 })),
        ]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has patterns containing exactly one entry {points=3, expectedIterations='1-2', confidence='medium'}
    let arr = data["patterns"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["points"].as_u64(), Some(3));
    assert_eq!(arr[0]["expectedIterations"].as_str(), Some("1-2"));
    assert_eq!(arr[0]["confidence"].as_str(), Some("medium"));
}

#[test]
fn scenario_dispatcher_four_done_units_yield_high_confidence() {
    // @step Given spec/work-units.json contains four done units all with estimate=5 and iterations 1, 2, 3, 4 respectively
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("A", "done", json!({ "estimate": 5, "iterations": 1 })),
            ("B", "done", json!({ "estimate": 5, "iterations": 2 })),
            ("C", "done", json!({ "estimate": 5, "iterations": 3 })),
            ("D", "done", json!({ "estimate": 5, "iterations": 4 })),
        ]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then the returned JSON has patterns containing exactly one entry {points=5, expectedIterations='1-4', confidence='high'}
    let arr = data["patterns"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["points"].as_u64(), Some(5));
    assert_eq!(arr[0]["expectedIterations"].as_str(), Some("1-4"));
    assert_eq!(arr[0]["confidence"].as_str(), Some("high"));
}

#[test]
fn scenario_dispatcher_two_buckets_sorted_ascending() {
    // @step Given spec/work-units.json contains two done units with estimate=5, iterations [1,2] and two done units with estimate=3, iterations [1,2]
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[
            ("A", "done", json!({ "estimate": 5, "iterations": 1 })),
            ("B", "done", json!({ "estimate": 5, "iterations": 2 })),
            ("C", "done", json!({ "estimate": 3, "iterations": 1 })),
            ("D", "done", json!({ "estimate": 3, "iterations": 2 })),
        ]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    let data = parse_data(&result.data);

    // @step Then patterns[0].points=3 and patterns[1].points=5
    let arr = data["patterns"].as_array().expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["points"].as_u64(), Some(3));
    assert_eq!(arr[1]["points"].as_u64(), Some(5));
}

#[test]
fn scenario_dispatcher_pattern_field_declaration_order() {
    // @step Given spec/work-units.json contains a single done unit with estimate=3 and iterations=1
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(&[("A", "done", json!({ "estimate": 3, "iterations": 1 }))]),
    );

    // @step When I dispatch query-estimation-guide with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));

    // @step Then the pattern object's field declaration order is points, expectedIterations, confidence
    let raw = &result.data;
    let expected = ["\"points\"", "\"expectedIterations\"", "\"confidence\""];
    let mut positions = Vec::new();
    for f in &expected {
        positions.push(raw.find(f).unwrap_or_else(|| panic!("missing {f}\n{raw}")));
    }
    for w in positions.windows(2) {
        assert!(
            w[0] < w[1],
            "field declaration order violated: {positions:?}\nraw:\n{raw}"
        );
    }
}

#[test]
fn scenario_dispatcher_malformed_work_units_json_yields_parse_error() {
    // @step Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), "{ not json");

    // @step When I dispatch query-estimation-guide against that project root with workUnitId='ANY-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'
    assert!(!result.success);
    let err = result.error.clone().unwrap_or_default();
    assert!(
        err.contains("Failed to parse work-units.json"),
        "expected error containing 'Failed to parse work-units.json'; got: {err}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// cli-subcommand scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_query_estimation_guide_with_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec query-estimation-guide --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("query-estimation-guide")
        .arg("--help")
        .output()
        .expect("spawn fspec query-estimation-guide --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains the substring 'query-estimation-guide'
    assert!(
        stdout.contains("query-estimation-guide") || stdout.contains("QUERY-ESTIMATION-GUIDE"),
        "help must describe subcommand; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_without_format_prints_nothing() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec query-estimation-guide ANY-001` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &["ANY-001"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout is exactly empty
    assert_eq!(stdout, "", "stdout must be empty (TS silent-text parity); got:\n{stdout}");

    // @step And stderr is exactly empty
    assert_eq!(stderr, "", "stderr must be empty; got:\n{stderr}");
}

#[test]
fn scenario_cli_format_text_prints_nothing() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec query-estimation-guide ANY-001 --format text` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &["ANY-001", "--format", "text"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout is exactly empty
    assert_eq!(stdout, "", "stdout must be empty for --format text; got:\n{stdout}");
}

#[test]
fn scenario_cli_format_json_empty_workspace_prints_empty_array() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec query-estimation-guide ANY-001 --format json` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &["ANY-001", "--format", "json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout parses as JSON whose root object has patterns=[]
    let parsed: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout must parse as JSON: {e}\nstdout:\n{stdout}"));
    assert_eq!(parsed["patterns"].as_array().map(|a| a.len()), Some(0));
}

#[test]
fn scenario_cli_requires_positional_work_unit_id() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec query-estimation-guide` from that directory with no positional argument
    let (code, stdout, stderr) = run_query(ws.path(), &[]);

    // @step Then the command exits with a non-zero code
    assert_ne!(code, 0, "missing required arg should exit non-zero; got {code}");
    // Sanity-check: stderr should mention either the required argument (green phase)
    // or unrecognized subcommand (red phase, before clap wiring). Either way, the
    // exit code must be non-zero and stderr must be non-empty.
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        !combined.trim().is_empty(),
        "expected non-empty stderr/stdout for missing-arg failure; got empty"
    );
}

#[test]
fn scenario_cli_malformed_work_units_json_exits_1_with_stderr() {
    // @step Given spec/work-units.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), "{ not json");

    // @step When I run `./codelet/target/release/fspec query-estimation-guide ANY-001 --format json` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &["ANY-001", "--format", "json"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1 on malformed; got {code}, stdout={stdout}, stderr={stderr}");
    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "expected 'Error:'; got:\n{stderr}");
    // @step And stderr contains the substring 'Failed to parse work-units.json'
    assert!(stderr.contains("Failed to parse work-units.json"), "got:\n{stderr}");
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a workspace whose spec/work-units.json contains a single done unit with estimate=3, iterations=1
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with(&[("A", "done", json!({ "estimate": 3, "iterations": 1 }))]),
    );

    // @step When I dispatch query-estimation-guide through fspec_core::dispatch::dispatch_command with workUnitId='ANY-001' and format='json' against that workspace
    let result = dispatch_command(req(
        ws.path(),
        json!({ "workUnitId": "ANY-001", "format": "json" }),
    ));
    assert!(result.success, "dispatcher must succeed");
    let dispatcher_data: Value = serde_json::from_str(&result.data).expect("JSON");

    // @step And I run `./codelet/target/release/fspec query-estimation-guide ANY-001 --format json` against the same workspace
    let (code, stdout, _) = run_query(ws.path(), &["ANY-001", "--format", "json"]);
    assert_eq!(code, 0, "binary path must exit 0");
    let binary_data: Value = serde_json::from_str(&stdout).expect("JSON");

    // @step Then both invocations produce JSON with patterns[0].points=3 and patterns[0].confidence='low'
    assert_eq!(dispatcher_data["patterns"][0]["points"].as_u64(), Some(3));
    assert_eq!(dispatcher_data["patterns"][0]["confidence"].as_str(), Some("low"));
    assert_eq!(binary_data["patterns"][0]["points"].as_u64(), Some(3));
    assert_eq!(binary_data["patterns"][0]["confidence"].as_str(), Some("low"));

    // @step And the CLI bridge module codelet/fspec/src/query_estimation_guide.rs contains NO inline grouping, bucketing, or rendering logic — its only computation is JSON arg marshalling and stdout printing
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/query_estimation_guide.rs");
    assert!(bridge_path.exists(), "bridge must exist: {}", bridge_path.display());
    let bridge_src = fs::read_to_string(&bridge_path).expect("read bridge");
    for forbidden in [
        "byPoints",
        "by_points",
        "EstimationPattern",
        "expectedIterations",
        "confidence",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}`; got:\n{bridge_src}"
        );
    }
}

const TS_HELP_FIXTURE_QEG: &str = include_str!("fixtures/help/query-estimation-guide.txt");

#[test]
fn scenario_query_estimation_guide_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec query-estimation-guide --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("query-estimation-guide")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn query-estimation-guide --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/query-estimation-guide.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_QEG);

    // @step And stdout starts with a blank line followed by 'QUERY-ESTIMATION-GUIDE'
    assert!(stdout.starts_with("\nQUERY-ESTIMATION-GUIDE\n"));
}

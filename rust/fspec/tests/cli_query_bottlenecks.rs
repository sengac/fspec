//! CLI surface for the `query-bottlenecks` subcommand on the standalone
//! fspec Rust binary — RPC-256.
//!
//! Feature: spec/features/query-bottlenecks-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand is wired (Phase C), these tests
//! exercise the binary and expect a NotYetPorted error path. Once the
//! subcommand is wired, the green-phase assertions take over.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_query(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("query-bottlenecks");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec query-bottlenecks");
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
        command: "query-bottlenecks".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

/// Build a work-units.json with provided entries.
/// `entries` is a slice of (id, status, extra_fields_json_obj).
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
// cli-subcommand scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_query_bottlenecks_with_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec query-bottlenecks --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("query-bottlenecks")
        .arg("--help")
        .output()
        .expect("spawn fspec query-bottlenecks --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec query-bottlenecks --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring 'query-bottlenecks'
    assert!(
        stdout.contains("query-bottlenecks") || stdout.contains("QUERY-BOTTLENECKS"),
        "help must describe the query-bottlenecks subcommand; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_no_options_empty_workspace_prints_success() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec query-bottlenecks` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains the exact line '✓ No bottlenecks found'
    assert!(
        stdout.contains("✓ No bottlenecks found"),
        "expected '✓ No bottlenecks found'; got stdout:\n{stdout}"
    );
}

#[test]
fn scenario_cli_output_json_empty_workspace_prints_empty_array() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec query-bottlenecks --output json` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &["--output", "json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout parses as JSON whose root object has bottlenecks=[]
    let parsed: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout must parse as JSON: {e}\nstdout:\n{stdout}"));
    assert_eq!(parsed["bottlenecks"].as_array().map(Vec::len), Some(0));
}

#[test]
fn scenario_cli_text_output_lists_qualifying_bottleneck() {
    // @step Given a workspace whose spec/work-units.json contains A with blocks=['B','C'] and B with blocks=['D'] and C and D with no dependency fields
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with(&[
            ("A", "backlog", json!({ "blocks": ["B", "C"] })),
            ("B", "backlog", json!({ "blocks": ["D"] })),
            ("C", "backlog", json!({})),
            ("D", "backlog", json!({})),
        ]),
    );

    // @step When I run `./rust/target/release/fspec query-bottlenecks` from that workspace
    let (code, stdout, stderr) = run_query(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains the substring 'Bottleneck Work Units (blocking 2+ work units):'
    assert!(
        stdout.contains("Bottleneck Work Units (blocking 2+ work units):"),
        "expected header; got:\n{stdout}"
    );
    // @step And stdout contains the substring 'A'
    assert!(stdout.contains("A"), "expected unit id A; got:\n{stdout}");
    // @step And stdout contains the substring 'Bottleneck Score: 3'
    assert!(
        stdout.contains("Bottleneck Score: 3"),
        "expected 'Bottleneck Score: 3'; got:\n{stdout}"
    );
    // @step And stdout contains the substring 'Total bottlenecks: 1'
    assert!(
        stdout.contains("Total bottlenecks: 1"),
        "expected 'Total bottlenecks: 1'; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_malformed_work_units_json_exits_1_with_stderr() {
    // @step Given spec/work-units.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), "{ not json");

    // @step When I run `./rust/target/release/fspec query-bottlenecks --output json` from that directory
    let (code, stdout, stderr) = run_query(ws.path(), &["--output", "json"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1 on malformed; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "expected 'Error:' in stderr; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Failed to parse work-units.json'
    assert!(
        stderr.contains("Failed to parse work-units.json"),
        "expected 'Failed to parse work-units.json'; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a workspace whose spec/work-units.json contains A with blocks=['B','C'] and B with blocks=['D']
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with(&[
            ("A", "backlog", json!({ "blocks": ["B", "C"] })),
            ("B", "backlog", json!({ "blocks": ["D"] })),
            ("C", "backlog", json!({})),
            ("D", "backlog", json!({})),
        ]),
    );

    // @step When I dispatch query-bottlenecks through fspec_core::dispatch::dispatch_command with output='json' against that workspace
    let result = dispatch_command(req(ws.path(), json!({ "output": "json" })));
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let dispatcher_data: Value = serde_json::from_str(&result.data).expect("dispatcher data JSON");

    // @step And I run `./rust/target/release/fspec query-bottlenecks --output json` against the same workspace
    let (code, stdout, _) = run_query(ws.path(), &["--output", "json"]);
    assert_eq!(code, 0, "binary path must exit 0");
    let binary_data: Value = serde_json::from_str(&stdout).expect("binary stdout JSON");

    // @step Then both invocations produce JSON with bottlenecks[0].id='A' and bottlenecks[0].score=3
    assert_eq!(dispatcher_data["bottlenecks"][0]["id"].as_str(), Some("A"));
    assert_eq!(dispatcher_data["bottlenecks"][0]["score"].as_u64(), Some(3));
    assert_eq!(binary_data["bottlenecks"][0]["id"].as_str(), Some("A"));
    assert_eq!(binary_data["bottlenecks"][0]["score"].as_u64(), Some(3));

    // @step And the CLI bridge module rust/fspec/src/query_bottlenecks.rs contains NO inline DFS, filtering, or rendering logic — its only computation is JSON arg marshalling and stdout printing
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/query_bottlenecks.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/query_bottlenecks.rs must exist as the CLI bridge module; missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "calculateBlockedWorkUnits",
        "calculate_blocked_work_units",
        "transitiveBlocks",
        "Bottleneck Score:",
        "Direct Blocks:",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

const TS_HELP_FIXTURE_QB: &str = include_str!("fixtures/help/query-bottlenecks.txt");

#[test]
fn scenario_query_bottlenecks_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec query-bottlenecks --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("query-bottlenecks")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn query-bottlenecks --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "query-bottlenecks --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/query-bottlenecks.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_QB);

    // @step And stdout starts with a blank line followed by 'QUERY-BOTTLENECKS'
    assert!(stdout.starts_with("\nQUERY-BOTTLENECKS\n"));
}

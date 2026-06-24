//! CLI surface for the `add-example` subcommand on the standalone fspec
//! Rust binary — RPC-181.
//!
//! Feature: spec/features/add-example-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand is wired (Phase C by supervisor),
//! these tests exercise the binary and expect either a NotYetPorted error
//! or a missing-subcommand failure. Once the subcommand is wired, the
//! green-phase assertions take over.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::{json, Value};
use tempfile::TempDir;

mod common;

use common::fspec_bin;

// ---------- helpers ----------

fn run_cmd(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-example");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-example");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn seed_one(cwd: &Path, id: &str, status: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let mut wu = serde_json::Map::new();
    wu.insert("id".into(), Value::String(id.to_string()));
    wu.insert("title".into(), Value::String(format!("title {id}")));
    wu.insert("type".into(), Value::String("story".into()));
    wu.insert("status".into(), Value::String(status.to_string()));
    wu.insert(
        "createdAt".into(),
        Value::String("2026-06-01T00:00:00.000Z".into()),
    );
    wu.insert(
        "updatedAt".into(),
        Value::String("2026-06-01T00:00:00.000Z".into()),
    );
    let mut wus = serde_json::Map::new();
    wus.insert(id.to_string(), Value::Object(wu));
    let mut state_arrays = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let arr = if *st == status {
            vec![Value::String(id.to_string())]
        } else {
            vec![]
        };
        state_arrays.insert((*st).to_string(), Value::Array(arr));
    }
    let v = json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": Value::Object(state_arrays),
    });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&v).unwrap(),
    )
    .expect("write work-units.json");
}

fn read_work_units(cwd: &Path) -> Value {
    let raw = fs::read_to_string(cwd.join("spec").join("work-units.json")).expect("read");
    serde_json::from_str(&raw).expect("parse")
}

// ---------- scenarios ----------

#[test]
fn help_output_matches_captured_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `fspec add-example --help`
    let output = Command::new(fspec_bin())
        .arg("add-example")
        .arg("--help")
        .output()
        .expect("spawn fspec add-example --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "fspec add-example --help must exit 0; got {code}");

    // @step And stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-example.txt
    let fixture = include_str!("fixtures/help/add-example.txt");
    assert_eq!(stdout, fixture, "help output must match TS fixture");
}

#[test]
fn happy_path_invocation_marshals_positional_args_and_writes_example() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    seed_one(tmp.path(), "AUTH-001", "specifying");

    // @step When I run `fspec add-example AUTH-001 "Valid login"` in that tempdir
    let (code, stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001", "Valid login"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring "✓ Example added successfully"
    assert!(
        stdout.contains("✓ Example added successfully"),
        "stdout: {stdout}"
    );

    // @step And stdout contains the substring "<system-reminder>"
    assert!(stdout.contains("<system-reminder>"), "stdout: {stdout}");

    // @step And spec/work-units.json on disk shows AUTH-001.examples has length 1
    let disk = read_work_units(tmp.path());
    let examples = disk["workUnits"]["AUTH-001"]["examples"]
        .as_array()
        .expect("examples array");
    assert_eq!(examples.len(), 1);
}

#[test]
fn missing_work_unit_exits_1_with_canonical_error_on_stderr() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    seed_one(tmp.path(), "AUTH-001", "specifying");

    // @step When I run `fspec add-example NOPE-001 "x"` in that tempdir
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["NOPE-001", "x"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring "✗ Failed to add example:"
    assert!(
        stderr.contains("✗ Failed to add example:"),
        "stderr: {stderr}"
    );

    // @step And stderr contains the substring "Work unit 'NOPE-001' does not exist"
    assert!(
        stderr.contains("Work unit 'NOPE-001' does not exist"),
        "stderr: {stderr}"
    );
}

#[test]
fn status_guard_exits_1_with_phase_guard_message_on_stderr() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    seed_one(tmp.path(), "AUTH-001", "backlog");

    // @step When I run `fspec add-example AUTH-001 "x"` in that tempdir
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001", "x"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring "Can only add examples during discovery/specification phase. AUTH-001 is in 'backlog' state."
    assert!(
        stderr.contains("Can only add examples during discovery/specification phase. AUTH-001 is in 'backlog' state."),
        "stderr: {stderr}"
    );
}

#[test]
fn cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let tmp_disp = TempDir::new().expect("tempdir-disp");
    seed_one(tmp_disp.path(), "AUTH-001", "specifying");
    let tmp_cli = TempDir::new().expect("tempdir-cli");
    seed_one(tmp_cli.path(), "AUTH-002", "specifying");

    // @step When I dispatch add-example via fspec_core::dispatch with workUnitId='AUTH-001' and example='X'
    let result = codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
        command: "add-example".to_string(),
        args_json: json!({"workUnitId": "AUTH-001", "example": "X"}).to_string(),
        project_root: tmp_disp.path().to_path_buf(),
    });
    assert!(result.success, "dispatcher success: {result:?}");

    // @step And I run the binary `fspec add-example AUTH-002 "Y"` against the same workspace shape
    let (code, _stdout, stderr) = run_cmd(tmp_cli.path(), &["AUTH-002", "Y"]);
    assert_eq!(code, 0, "binary success; stderr={stderr}");

    // @step Then both invocations call commands::add_example::run with the same JSON-marshalled args
    // (Indirectly verified: both produce exactly one new example each.)

    // @step And the resulting spec/work-units.json contains exactly one new example per call
    let disk_disp = read_work_units(tmp_disp.path());
    let disk_cli = read_work_units(tmp_cli.path());
    let ex_disp = disk_disp["workUnits"]["AUTH-001"]["examples"]
        .as_array()
        .expect("dispatcher examples");
    let ex_cli = disk_cli["workUnits"]["AUTH-002"]["examples"]
        .as_array()
        .expect("cli examples");
    assert_eq!(ex_disp.len(), 1);
    assert_eq!(ex_cli.len(), 1);
}

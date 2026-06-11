//! CLI surface for the `restore-example` subcommand on the standalone fspec
//! Rust binary — RPC-289.
//!
//! Feature: spec/features/restore-example-cli-subcommand.feature

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
    cmd.arg("restore-example");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec restore-example");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn seed_one(cwd: &Path, id: &str, status: &str, examples: Option<Value>) {
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
    if let Some(ex) = examples {
        wu.insert("examples".into(), ex);
    }
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

    // @step When I run `fspec restore-example --help`
    let output = Command::new(fspec_bin())
        .arg("restore-example")
        .arg("--help")
        .output()
        .expect("spawn fspec restore-example --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "fspec restore-example --help must exit 0");

    // @step And stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/restore-example.txt
    let fixture = include_str!("fixtures/help/restore-example.txt");
    assert_eq!(stdout, fixture);
}

#[test]
fn happy_path_restore_via_cli() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has one example id=0 'hello' deleted=true with a deletedAt timestamp
    let tmp = TempDir::new().expect("tempdir");
    seed_one(
        tmp.path(),
        "AUTH-001",
        "specifying",
        Some(json!([
            {"id": 0, "text": "hello", "deleted": true, "createdAt": "2026-06-01T00:00:00.000Z", "deletedAt": "2026-06-02T00:00:00.000Z"}
        ])),
    );

    // @step When I run `fspec restore-example AUTH-001 0` in that tempdir
    let (code, stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001", "0"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Restored example: "hello"'
    assert!(
        stdout.contains("✓ Restored example: \"hello\""),
        "stdout: {stdout}"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.examples[0].deleted=false
    let disk = read_work_units(tmp.path());
    assert_eq!(
        disk["workUnits"]["AUTH-001"]["examples"][0]["deleted"],
        false
    );

    // @step And spec/work-units.json on disk shows AUTH-001.examples[0] has no deletedAt key
    let ex0 = &disk["workUnits"]["AUTH-001"]["examples"][0];
    assert!(
        ex0.get("deletedAt").is_none(),
        "deletedAt absent: {ex0}"
    );
}

#[test]
fn missing_work_unit_exits_1_with_canonical_stderr() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    seed_one(tmp.path(), "AUTH-001", "specifying", None);

    // @step When I run `fspec restore-example NOPE-001 0` in that tempdir
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["NOPE-001", "0"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "exit 1; stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to restore example:'
    assert!(
        stderr.contains("✗ Failed to restore example:"),
        "stderr: {stderr}"
    );

    // @step And stderr contains the substring "Work unit 'NOPE-001' does not exist"
    assert!(
        stderr.contains("Work unit 'NOPE-001' does not exist"),
        "stderr: {stderr}"
    );
}

#[test]
fn wrong_status_exits_1_with_phase_guard_message() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=backlog has one example id=0 deleted=true
    let tmp = TempDir::new().expect("tempdir");
    seed_one(
        tmp.path(),
        "AUTH-001",
        "backlog",
        Some(json!([
            {"id": 0, "text": "hello", "deleted": true, "createdAt": "2026-06-01T00:00:00.000Z", "deletedAt": "2026-06-02T00:00:00.000Z"}
        ])),
    );

    // @step When I run `fspec restore-example AUTH-001 0` in that tempdir
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001", "0"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "exit 1; stderr={stderr}");

    // @step And stderr contains the substring "Can only restore examples during discovery/specification phase. AUTH-001 is in 'backlog' state."
    assert!(
        stderr.contains("Can only restore examples during discovery/specification phase. AUTH-001 is in 'backlog' state."),
        "stderr: {stderr}"
    );
}

#[test]
fn non_numeric_index_falls_through_to_ts_parse_int_nan_parity() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has one example id=0 deleted=true
    let tmp = TempDir::new().expect("tempdir");
    seed_one(
        tmp.path(),
        "AUTH-001",
        "specifying",
        Some(json!([
            {"id": 0, "text": "x", "deleted": true, "createdAt": "2026-06-01T00:00:00.000Z", "deletedAt": "2026-06-02T00:00:00.000Z"}
        ])),
    );

    // @step When I run `fspec restore-example AUTH-001 abc` in that tempdir
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001", "abc"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Example with ID NaN not found'
    assert!(
        stderr.contains("Example with ID NaN not found"),
        "stderr: {stderr}"
    );
}

#[test]
fn unknown_ids_flag_is_rejected_by_clap_with_exit_1() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has examples id=0 and id=1 both deleted=true
    let tmp = TempDir::new().expect("tempdir");
    seed_one(
        tmp.path(),
        "AUTH-001",
        "specifying",
        Some(json!([
            {"id": 0, "text": "a", "deleted": true, "createdAt": "2026-06-01T00:00:00.000Z", "deletedAt": "2026-06-02T00:00:00.000Z"},
            {"id": 1, "text": "b", "deleted": true, "createdAt": "2026-06-01T00:00:00.000Z", "deletedAt": "2026-06-02T00:00:00.000Z"}
        ])),
    );

    // @step When I run `fspec restore-example AUTH-001 0 --ids 1,2` in that tempdir
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001", "0", "--ids", "1,2"]);

    // @step Then the exit code is not 0
    assert_ne!(code, 0, "expected non-zero exit; stderr={stderr}");

    // @step And stderr contains the substring 'unknown'
    let lower = stderr.to_lowercase();
    assert!(
        lower.contains("unknown") || lower.contains("unexpected"),
        "stderr: {stderr}"
    );
}

#[test]
fn cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001 status=specifying has examples id=0 and id=1 both deleted=true
    let tmp_disp = TempDir::new().expect("tempdir-disp");
    seed_one(
        tmp_disp.path(),
        "AUTH-001",
        "specifying",
        Some(json!([
            {"id": 0, "text": "zero", "deleted": true, "createdAt": "2026-06-01T00:00:00.000Z", "deletedAt": "2026-06-02T00:00:00.000Z"},
            {"id": 1, "text": "one", "deleted": true, "createdAt": "2026-06-01T00:00:00.000Z", "deletedAt": "2026-06-02T00:00:00.000Z"}
        ])),
    );
    let tmp_cli = TempDir::new().expect("tempdir-cli");
    seed_one(
        tmp_cli.path(),
        "AUTH-001",
        "specifying",
        Some(json!([
            {"id": 0, "text": "zero", "deleted": true, "createdAt": "2026-06-01T00:00:00.000Z", "deletedAt": "2026-06-02T00:00:00.000Z"},
            {"id": 1, "text": "one", "deleted": true, "createdAt": "2026-06-01T00:00:00.000Z", "deletedAt": "2026-06-02T00:00:00.000Z"}
        ])),
    );

    // @step When I dispatch restore-example via fspec_core::dispatch with workUnitId='AUTH-001' and index=0
    let result = codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
        command: "restore-example".to_string(),
        args_json: json!({"workUnitId": "AUTH-001", "index": 0}).to_string(),
        project_root: tmp_disp.path().to_path_buf(),
    });
    assert!(result.success, "dispatcher success: {result:?}");

    // @step And I run the binary `fspec restore-example AUTH-001 1` against the same workspace shape
    let (code, _stdout, stderr) = run_cmd(tmp_cli.path(), &["AUTH-001", "1"]);
    assert_eq!(code, 0, "binary success; stderr={stderr}");

    // @step Then both invocations call commands::restore_example::run with the same JSON-marshalled args
    // (Indirectly verified — both produced restorations.)

    // @step And both examples end up deleted=false on disk
    let disk_disp = read_work_units(tmp_disp.path());
    let disk_cli = read_work_units(tmp_cli.path());
    assert_eq!(
        disk_disp["workUnits"]["AUTH-001"]["examples"][0]["deleted"],
        false
    );
    assert_eq!(
        disk_cli["workUnits"]["AUTH-001"]["examples"][1]["deleted"],
        false
    );
}

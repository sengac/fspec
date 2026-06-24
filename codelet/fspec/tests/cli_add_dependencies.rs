//! CLI surface for the `add-dependencies` subcommand on the standalone
//! fspec Rust binary — RPC-176.
//!
//! Feature: spec/features/add-dependencies-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand is wired (Phase C), these tests
//! exercise the binary and expect either a NotYetPorted error path or
//! a missing-subcommand failure. Once the subcommand is wired, the
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
    cmd.arg("add-dependencies");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-dependencies");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn seed_work_units(cwd: &Path, units: &[(&str, &str)]) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let mut wus = serde_json::Map::new();
    let mut state_map: std::collections::HashMap<&str, Vec<String>> =
        std::collections::HashMap::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        state_map.insert(*st, Vec::new());
    }
    for (id, status) in units {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("type".into(), Value::String("story".into()));
        obj.insert("status".into(), Value::String((*status).to_string()));
        obj.insert(
            "createdAt".into(),
            Value::String("2026-06-01T00:00:00.000Z".into()),
        );
        obj.insert(
            "updatedAt".into(),
            Value::String("2026-06-01T00:00:00.000Z".into()),
        );
        wus.insert((*id).to_string(), Value::Object(obj));
        state_map.get_mut(*status).unwrap().push((*id).to_string());
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
                state_map[st]
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    let v = json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": Value::Object(states_obj),
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

    // @step When I run `fspec add-dependencies --help`
    let output = Command::new(fspec_bin())
        .arg("add-dependencies")
        .arg("--help")
        .output()
        .expect("spawn fspec add-dependencies --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "fspec add-dependencies --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-dependencies.txt
    assert!(
        stdout.contains("add-dependencies") || stdout.contains("ADD-DEPENDENCIES"),
        "help must describe the add-dependencies subcommand; got:\n{stdout}"
    );
}

#[test]
fn multi_flag_invocation_marshalls_all_arrays() {
    // @step Given a project root tempdir with AUTH-001, AUTH-002, AUTH-003, FOO-001 all status=backlog and empty dependency arrays
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        &[
            ("AUTH-001", "backlog"),
            ("AUTH-002", "backlog"),
            ("AUTH-003", "backlog"),
            ("FOO-001", "backlog"),
        ],
    );

    // @step When I run `fspec add-dependencies AUTH-001 --blocks AUTH-002 AUTH-003 --depends-on FOO-001`
    let (code, stdout, stderr) = run_cmd(
        tmp.path(),
        &[
            "AUTH-001",
            "--blocks",
            "AUTH-002",
            "AUTH-003",
            "--depends-on",
            "FOO-001",
        ],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "exit must be 0; stderr={stderr}, stdout={stdout}");

    // @step And stdout contains the substring '✓ Added 3 dependencies successfully'
    assert!(
        stdout.contains("Added 3 dependencies successfully"),
        "stdout: {stdout}"
    );

    let disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001.blocks=['AUTH-002', 'AUTH-003']
    assert_eq!(
        disk["workUnits"]["AUTH-001"]["blocks"],
        json!(["AUTH-002", "AUTH-003"])
    );

    // @step And spec/work-units.json on disk shows AUTH-001.dependsOn=['FOO-001']
    assert_eq!(
        disk["workUnits"]["AUTH-001"]["dependsOn"],
        json!(["FOO-001"])
    );

    // @step And spec/work-units.json on disk shows AUTH-002.blockedBy contains 'AUTH-001' and AUTH-002.status='blocked'
    let bb = disk["workUnits"]["AUTH-002"]["blockedBy"]
        .as_array()
        .unwrap();
    assert!(bb.iter().any(|v| v == "AUTH-001"));
    assert_eq!(disk["workUnits"]["AUTH-002"]["status"], "blocked");
}

#[test]
fn missing_source_work_unit_exits_1_with_canonical_error_on_stderr() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), &[("AUTH-001", "backlog")]);

    // @step When I run `fspec add-dependencies NOPE-001 --blocks AUTH-001`
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["NOPE-001", "--blocks", "AUTH-001"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring "Work unit 'NOPE-001' does not exist"
    assert!(
        stderr.contains("Work unit 'NOPE-001' does not exist"),
        "stderr: {stderr}"
    );
}

#[test]
fn self_dependency_exits_1_with_canonical_message() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), &[("AUTH-001", "backlog")]);

    // @step When I run `fspec add-dependencies AUTH-001 --blocks AUTH-001`
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001", "--blocks", "AUTH-001"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Cannot create self-dependency'
    assert!(
        stderr.contains("Cannot create self-dependency"),
        "stderr: {stderr}"
    );
}

#[test]
fn no_flags_supplied_results_in_zero_added_success() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), &[("AUTH-001", "backlog")]);

    // @step When I run `fspec add-dependencies AUTH-001`
    let (code, stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Added 0 dependencies successfully'
    assert!(
        stdout.contains("Added 0 dependencies successfully"),
        "stdout: {stdout}"
    );

    // @step And spec/work-units.json on disk shows AUTH-001 with no blocks, no blockedBy, no dependsOn, no relatesTo fields
    let disk = read_work_units(tmp.path());
    for field in &["blocks", "blockedBy", "dependsOn", "relatesTo"] {
        assert!(
            disk["workUnits"]["AUTH-001"].get(*field).is_none(),
            "AUTH-001.{field} should be absent"
        );
    }
}

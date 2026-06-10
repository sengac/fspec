//! CLI surface for the `clear-dependencies` subcommand on the standalone
//! fspec Rust binary — RPC-204.
//!
//! Feature: spec/features/clear-dependencies-cli-subcommand.feature
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
    cmd.arg("clear-dependencies");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec clear-dependencies");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn seed_work_units(cwd: &Path, units: &[(&str, &str)]) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let mut wus = serde_json::Map::new();
    let mut state_map: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();
    for st in &["backlog", "specifying", "testing", "implementing", "validating", "done", "blocked"] {
        state_map.insert(*st, Vec::new());
    }
    for (id, status) in units {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), Value::String((*id).to_string()));
        obj.insert("title".into(), Value::String(format!("title {id}")));
        obj.insert("type".into(), Value::String("story".into()));
        obj.insert("status".into(), Value::String((*status).to_string()));
        obj.insert("createdAt".into(), Value::String("2026-06-01T00:00:00.000Z".into()));
        obj.insert("updatedAt".into(), Value::String("2026-06-01T00:00:00.000Z".into()));
        wus.insert((*id).to_string(), Value::Object(obj));
        state_map.get_mut(*status).unwrap().push((*id).to_string());
    }
    let mut states_obj = serde_json::Map::new();
    for st in &["backlog", "specifying", "testing", "implementing", "validating", "done", "blocked"] {
        states_obj.insert(
            (*st).to_string(),
            Value::Array(state_map[st].iter().map(|s| Value::String(s.clone())).collect()),
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

fn set_field(cwd: &Path, id: &str, field: &str, values: &[&str]) {
    let path = cwd.join("spec").join("work-units.json");
    let raw = fs::read_to_string(&path).expect("read work-units.json");
    let mut v: Value = serde_json::from_str(&raw).expect("parse");
    let arr: Vec<Value> = values.iter().map(|s| Value::String((*s).to_string())).collect();
    v["workUnits"][id][field] = Value::Array(arr);
    fs::write(&path, serde_json::to_string_pretty(&v).unwrap()).expect("write");
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

    // @step When I run `fspec clear-dependencies --help`
    let output = Command::new(fspec_bin())
        .arg("clear-dependencies")
        .arg("--help")
        .output()
        .expect("spawn fspec clear-dependencies --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "fspec clear-dependencies --help must exit 0; got {code}, stderr={stderr}");

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/clear-dependencies.txt
    assert!(
        stdout.contains("clear-dependencies") || stdout.contains("CLEAR-DEPENDENCIES"),
        "help must describe the clear-dependencies subcommand; got:\n{stdout}"
    );
}

#[test]
fn invocation_with_confirm_wipes_every_dependency_edge() {
    // @step Given a project root tempdir with AUTH-001 having blocks=['AUTH-002'] dependsOn=['API-001'] relatesTo=['UI-001'], AUTH-002.blockedBy=['AUTH-001'], API-001, UI-001.relatesTo=['AUTH-001']
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        &[
            ("AUTH-001", "backlog"),
            ("AUTH-002", "backlog"),
            ("API-001", "backlog"),
            ("UI-001", "backlog"),
        ],
    );
    set_field(tmp.path(), "AUTH-001", "blocks", &["AUTH-002"]);
    set_field(tmp.path(), "AUTH-001", "dependsOn", &["API-001"]);
    set_field(tmp.path(), "AUTH-001", "relatesTo", &["UI-001"]);
    set_field(tmp.path(), "AUTH-002", "blockedBy", &["AUTH-001"]);
    set_field(tmp.path(), "UI-001", "relatesTo", &["AUTH-001"]);

    // @step When I run `fspec clear-dependencies AUTH-001 --confirm`
    let (code, stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001", "--confirm"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}, stdout={stdout}");

    // @step And stdout contains the substring '✓ All dependencies cleared from AUTH-001'
    assert!(
        stdout.contains("All dependencies cleared from AUTH-001"),
        "stdout: {stdout}"
    );

    let disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001 has no blocks, blockedBy, dependsOn, or relatesTo fields
    for field in &["blocks", "blockedBy", "dependsOn", "relatesTo"] {
        assert!(
            disk["workUnits"]["AUTH-001"].get(*field).is_none(),
            "AUTH-001.{field} should be absent"
        );
    }

    // @step And spec/work-units.json on disk shows AUTH-002 has no blockedBy field
    assert!(
        disk["workUnits"]["AUTH-002"].get("blockedBy").is_none(),
        "AUTH-002.blockedBy should be absent"
    );

    // @step And spec/work-units.json on disk shows UI-001 has no relatesTo field
    assert!(
        disk["workUnits"]["UI-001"].get("relatesTo").is_none(),
        "UI-001.relatesTo should be absent"
    );
}

#[test]
fn missing_confirm_flag_exits_1_with_canonical_error_on_stderr() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog with blocks=['AUTH-002']
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        &[("AUTH-001", "backlog"), ("AUTH-002", "backlog")],
    );
    set_field(tmp.path(), "AUTH-001", "blocks", &["AUTH-002"]);

    // @step When I run `fspec clear-dependencies AUTH-001`
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Must confirm clearing all dependencies with --confirm flag'
    assert!(
        stderr.contains("Must confirm clearing all dependencies with --confirm flag"),
        "stderr: {stderr}"
    );
}

#[test]
fn missing_source_work_unit_exits_1_with_canonical_error_on_stderr() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), &[("AUTH-001", "backlog")]);

    // @step When I run `fspec clear-dependencies UNKNOWN-001 --confirm`
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["UNKNOWN-001", "--confirm"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring "Work unit 'UNKNOWN-001' does not exist"
    assert!(
        stderr.contains("Work unit 'UNKNOWN-001' does not exist"),
        "stderr: {stderr}"
    );
}

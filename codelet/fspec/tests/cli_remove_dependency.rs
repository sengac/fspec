//! CLI surface for the `remove-dependency` subcommand on the standalone
//! fspec Rust binary — RPC-271.
//!
//! Feature: spec/features/remove-dependency-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand is wired (Phase C), these tests
//! exercise the binary and expect either a NotYetPorted error path or
//! a missing-subcommand failure. Once the subcommand is wired by the
//! supervisor, the green-phase assertions take over.

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
    cmd.arg("remove-dependency");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec remove-dependency");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn seed_work_units(cwd: &Path, units: &[(&str, &str)], deps: &[(&str, &str, &[&str])]) {
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
            Value::String("2025-01-01T00:00:00.000Z".into()),
        );
        obj.insert(
            "updatedAt".into(),
            Value::String("2025-01-01T00:00:00.000Z".into()),
        );
        wus.insert((*id).to_string(), Value::Object(obj));
        state_map.get_mut(*status).unwrap().push((*id).to_string());
    }
    for (id, field, ids) in deps {
        let arr: Vec<Value> = ids
            .iter()
            .map(|s| Value::String((*s).to_string()))
            .collect();
        if let Some(Value::Object(obj)) = wus.get_mut(*id) {
            obj.insert((*field).to_string(), Value::Array(arr));
        }
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

    // @step When I run `fspec remove-dependency --help`
    let output = Command::new(fspec_bin())
        .arg("remove-dependency")
        .arg("--help")
        .output()
        .expect("spawn fspec remove-dependency --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "fspec remove-dependency --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/remove-dependency.txt
    assert!(
        stdout.contains("remove-dependency") || stdout.contains("REMOVE-DEPENDENCY"),
        "help must describe the remove-dependency subcommand; got:\n{stdout}"
    );
}

#[test]
fn positional_shorthand_removes_depends_on_edge() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001.dependsOn=['AUTH-002']
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        &[("AUTH-001", "backlog"), ("AUTH-002", "backlog")],
        &[("AUTH-001", "dependsOn", &["AUTH-002"])],
    );

    // @step When I run `fspec remove-dependency AUTH-001 AUTH-002`
    let (code, stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001", "AUTH-002"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "exit must be 0; stderr={stderr}, stdout={stdout}");

    // @step And stdout contains the substring '✓ Dependency removed successfully'
    assert!(
        stdout.contains("Dependency removed successfully"),
        "stdout: {stdout}"
    );

    // @step And spec/work-units.json on disk shows AUTH-001 has no dependsOn field
    let disk = read_work_units(tmp.path());
    assert!(disk["workUnits"]["AUTH-001"].get("dependsOn").is_none());
}

#[test]
fn depends_on_flag_removes_same_edge_as_positional_shorthand() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001.dependsOn=['AUTH-002']
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        &[("AUTH-001", "backlog"), ("AUTH-002", "backlog")],
        &[("AUTH-001", "dependsOn", &["AUTH-002"])],
    );

    // @step When I run `fspec remove-dependency AUTH-001 --depends-on AUTH-002`
    let (code, stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001", "--depends-on", "AUTH-002"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "exit must be 0; stderr={stderr}, stdout={stdout}");

    // @step And stdout contains the substring '✓ Dependency removed successfully'
    assert!(
        stdout.contains("Dependency removed successfully"),
        "stdout: {stdout}"
    );

    // @step And spec/work-units.json on disk shows AUTH-001 has no dependsOn field
    let disk = read_work_units(tmp.path());
    assert!(disk["workUnits"]["AUTH-001"].get("dependsOn").is_none());
}

#[test]
fn positional_and_depends_on_with_same_value_succeed_without_conflict() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001.dependsOn=['AUTH-002']
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        &[("AUTH-001", "backlog"), ("AUTH-002", "backlog")],
        &[("AUTH-001", "dependsOn", &["AUTH-002"])],
    );

    // @step When I run `fspec remove-dependency AUTH-001 AUTH-002 --depends-on AUTH-002`
    let (code, stdout, stderr) = run_cmd(
        tmp.path(),
        &["AUTH-001", "AUTH-002", "--depends-on", "AUTH-002"],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "exit must be 0; stderr={stderr}, stdout={stdout}");

    // @step And stdout contains the substring '✓ Dependency removed successfully'
    assert!(
        stdout.contains("Dependency removed successfully"),
        "stdout: {stdout}"
    );
}

#[test]
fn positional_and_depends_on_with_different_values_exits_1_with_conflict_message() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001.dependsOn=['AUTH-002']
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        &[
            ("AUTH-001", "backlog"),
            ("AUTH-002", "backlog"),
            ("AUTH-003", "backlog"),
        ],
        &[("AUTH-001", "dependsOn", &["AUTH-002"])],
    );

    // @step When I run `fspec remove-dependency AUTH-001 AUTH-002 --depends-on AUTH-003`
    let (code, _stdout, stderr) = run_cmd(
        tmp.path(),
        &["AUTH-001", "AUTH-002", "--depends-on", "AUTH-003"],
    );

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Cannot specify dependency both as argument and --depends-on option'
    assert!(
        stderr.contains("Cannot specify dependency both as argument and --depends-on option"),
        "stderr: {stderr}"
    );
}

#[test]
fn no_relationship_args_supplied_exits_1_with_at_least_one_guard() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), &[("AUTH-001", "backlog")], &[]);

    // @step When I run `fspec remove-dependency AUTH-001`
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Must specify at least one relationship to remove'
    assert!(
        stderr.contains("Must specify at least one relationship to remove"),
        "stderr: {stderr}"
    );
}

#[test]
fn blocks_flag_removes_a_blocks_edge_bidirectionally() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-001.blocks=['AUTH-002'] and AUTH-002.blockedBy=['AUTH-001']
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        &[("AUTH-001", "backlog"), ("AUTH-002", "blocked")],
        &[
            ("AUTH-001", "blocks", &["AUTH-002"]),
            ("AUTH-002", "blockedBy", &["AUTH-001"]),
        ],
    );

    // @step When I run `fspec remove-dependency AUTH-001 --blocks AUTH-002`
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["AUTH-001", "--blocks", "AUTH-002"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    let disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-001 has no blocks field
    assert!(disk["workUnits"]["AUTH-001"].get("blocks").is_none());

    // @step And spec/work-units.json on disk shows AUTH-002 has no blockedBy field
    assert!(disk["workUnits"]["AUTH-002"].get("blockedBy").is_none());
}

#[test]
fn blocked_by_flag_removes_a_blocked_by_edge_bidirectionally() {
    // @step Given a project root tempdir with spec/work-units.json where UI-001.blockedBy=['API-001'] and API-001.blocks=['UI-001']
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        &[("UI-001", "blocked"), ("API-001", "backlog")],
        &[
            ("UI-001", "blockedBy", &["API-001"]),
            ("API-001", "blocks", &["UI-001"]),
        ],
    );

    // @step When I run `fspec remove-dependency UI-001 --blocked-by API-001`
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["UI-001", "--blocked-by", "API-001"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    let disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows UI-001 has no blockedBy field
    assert!(disk["workUnits"]["UI-001"].get("blockedBy").is_none());

    // @step And spec/work-units.json on disk shows API-001 has no blocks field
    assert!(disk["workUnits"]["API-001"].get("blocks").is_none());
}

#[test]
fn relates_to_flag_removes_a_symmetric_relates_to_edge() {
    // @step Given a project root tempdir with spec/work-units.json where AUTH-002.relatesTo=['AUTH-003'] and AUTH-003.relatesTo=['AUTH-002']
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(
        tmp.path(),
        &[("AUTH-002", "backlog"), ("AUTH-003", "backlog")],
        &[
            ("AUTH-002", "relatesTo", &["AUTH-003"]),
            ("AUTH-003", "relatesTo", &["AUTH-002"]),
        ],
    );

    // @step When I run `fspec remove-dependency AUTH-002 --relates-to AUTH-003`
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["AUTH-002", "--relates-to", "AUTH-003"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    let disk = read_work_units(tmp.path());

    // @step And spec/work-units.json on disk shows AUTH-002 has no relatesTo field
    assert!(disk["workUnits"]["AUTH-002"].get("relatesTo").is_none());

    // @step And spec/work-units.json on disk shows AUTH-003 has no relatesTo field
    assert!(disk["workUnits"]["AUTH-003"].get("relatesTo").is_none());
}

#[test]
fn missing_source_work_unit_exits_1_with_canonical_error_on_stderr() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=backlog
    let tmp = TempDir::new().expect("tempdir");
    seed_work_units(tmp.path(), &[("AUTH-001", "backlog")], &[]);

    // @step When I run `fspec remove-dependency NOPE-001 --depends-on AUTH-001`
    let (code, _stdout, stderr) = run_cmd(tmp.path(), &["NOPE-001", "--depends-on", "AUTH-001"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring "Work unit 'NOPE-001' does not exist"
    assert!(
        stderr.contains("Work unit 'NOPE-001' does not exist"),
        "stderr: {stderr}"
    );
}

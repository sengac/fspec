//! CLI surface for the `add-dependency` subcommand on the standalone fspec
//! Rust binary — RPC-177.
//!
//! Feature: spec/features/add-dependency-cli-subcommand.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

fn run_add_dep(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-dependency");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-dependency");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units(project_root: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn seed_units(units: &[(&str, &str)]) -> String {
    let mut wus = serde_json::Map::new();
    let mut states: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        states.insert(*st, Vec::new());
    }
    for (id, status) in units {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), serde_json::Value::String((*id).to_string()));
        obj.insert(
            "title".into(),
            serde_json::Value::String(format!("title {id}")),
        );
        obj.insert(
            "type".into(),
            serde_json::Value::String("story".to_string()),
        );
        obj.insert(
            "status".into(),
            serde_json::Value::String((*status).to_string()),
        );
        obj.insert(
            "createdAt".into(),
            serde_json::Value::String("2026-06-01T00:00:00.000Z".to_string()),
        );
        obj.insert(
            "updatedAt".into(),
            serde_json::Value::String("2026-06-01T00:00:00.000Z".to_string()),
        );
        wus.insert((*id).to_string(), serde_json::Value::Object(obj));
        states
            .get_mut(*status)
            .expect("known state")
            .push((*id).to_string());
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
            serde_json::Value::Array(
                states
                    .get(*st)
                    .expect("seeded state")
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    serde_json::to_string_pretty(&serde_json::json!({
        "version": "0.7.1",
        "workUnits": serde_json::Value::Object(wus),
        "states": serde_json::Value::Object(states_obj),
    }))
    .unwrap()
}

const TS_HELP_FIXTURE_AD: &str = include_str!("fixtures/help/add-dependency.txt");

#[test]
fn scenario_add_dependency_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec add-dependency --help`
    let output = Command::new(fspec_bin())
        .arg("add-dependency")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-dependency --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "add-dependency --help must exit 0; stderr={stderr}"
    );

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-dependency.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_AD);

    // @step And stdout starts with a blank line followed by 'ADD-DEPENDENCY'
    assert!(
        stdout.starts_with("\nADD-DEPENDENCY\n"),
        "got start: {:?}",
        &stdout[..stdout.len().min(40)]
    );
}

#[test]
fn scenario_cli_successfully_adds_shorthand_depends_on() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 and AUTH-002 both status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &seed_units(&[("AUTH-001", "specifying"), ("AUTH-002", "specifying")]),
    );

    // @step When I run `fspec add-dependency AUTH-002 AUTH-001` in that tempdir
    let (code, stdout, stderr) = run_add_dep(ws.path(), &["AUTH-002", "AUTH-001"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Dependency added successfully'
    assert!(
        stdout.contains("✓ Dependency added successfully"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And spec/work-units.json on disk shows AUTH-002.dependsOn=['AUTH-001']
    let v = read_work_units(ws.path());
    let deps = v["workUnits"]["AUTH-002"]["dependsOn"]
        .as_array()
        .expect("dependsOn array");
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].as_str(), Some("AUTH-001"));
}

#[test]
fn scenario_cli_successfully_adds_blocks_edge() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 and API-001 both status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &seed_units(&[("AUTH-001", "specifying"), ("API-001", "specifying")]),
    );

    // @step When I run `fspec add-dependency AUTH-001 --blocks API-001` in that tempdir
    let (code, stdout, stderr) = run_add_dep(ws.path(), &["AUTH-001", "--blocks", "API-001"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Dependency added successfully'
    assert!(
        stdout.contains("✓ Dependency added successfully"),
        "got: {stdout}"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.blocks=['API-001']
    let v = read_work_units(ws.path());
    let blocks = v["workUnits"]["AUTH-001"]["blocks"]
        .as_array()
        .expect("blocks array");
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].as_str(), Some("API-001"));
    // @step And spec/work-units.json on disk shows API-001.status='blocked'
    assert_eq!(
        v["workUnits"]["API-001"]["status"].as_str(),
        Some("blocked")
    );
}

#[test]
fn scenario_cli_rejects_invocation_with_no_relationship_args() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_units(&[("AUTH-001", "specifying")]));
    let pre_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();

    // @step When I run `fspec add-dependency AUTH-001` in that tempdir
    let (code, _stdout, stderr) = run_add_dep(ws.path(), &["AUTH-001"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to add dependency:'
    assert!(
        stderr.contains("✗ Failed to add dependency:"),
        "got stderr: {stderr}"
    );
    // @step And stderr contains the substring 'Must specify at least one relationship'
    assert!(
        stderr.contains("Must specify at least one relationship"),
        "got stderr: {stderr}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn scenario_cli_rejects_conflict_between_shorthand_and_depends_on() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001, AUTH-002, AUTH-003 all status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &seed_units(&[
            ("AUTH-001", "specifying"),
            ("AUTH-002", "specifying"),
            ("AUTH-003", "specifying"),
        ]),
    );
    let pre_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();

    // @step When I run `fspec add-dependency AUTH-001 AUTH-002 --depends-on AUTH-003` in that tempdir
    let (code, _stdout, stderr) = run_add_dep(
        ws.path(),
        &["AUTH-001", "AUTH-002", "--depends-on", "AUTH-003"],
    );

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");
    // @step And stderr contains the substring '✗ Failed to add dependency:'
    assert!(
        stderr.contains("✗ Failed to add dependency:"),
        "got stderr: {stderr}"
    );
    // @step And stderr contains the substring 'Cannot specify dependency both as argument and --depends-on option'
    assert!(
        stderr.contains("Cannot specify dependency both as argument and --depends-on option"),
        "got stderr: {stderr}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn scenario_cli_rejects_circular_blocks_with_exit_1() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 with blocks=['AUTH-002'] and AUTH-002 blockedBy=['AUTH-001']
    let ws = tempfile::tempdir().expect("tempdir");
    let mut pre: serde_json::Value = serde_json::from_str(&seed_units(&[
        ("AUTH-001", "specifying"),
        ("AUTH-002", "blocked"),
    ]))
    .unwrap();
    pre["workUnits"]["AUTH-001"]["blocks"] = serde_json::json!(["AUTH-002"]);
    pre["workUnits"]["AUTH-002"]["blockedBy"] = serde_json::json!(["AUTH-001"]);
    write_work_units(ws.path(), &serde_json::to_string_pretty(&pre).unwrap());
    let pre_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();

    // @step When I run `fspec add-dependency AUTH-002 --blocks AUTH-001` in that tempdir
    let (code, _stdout, stderr) = run_add_dep(ws.path(), &["AUTH-002", "--blocks", "AUTH-001"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");
    // @step And stderr contains the substring '✗ Failed to add dependency:'
    assert!(
        stderr.contains("✗ Failed to add dependency:"),
        "got: {stderr}"
    );
    // @step And stderr contains the substring 'Circular dependency detected'
    assert!(
        stderr.contains("Circular dependency detected"),
        "got: {stderr}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 and AUTH-002 both status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &seed_units(&[("AUTH-001", "specifying"), ("AUTH-002", "specifying")]),
    );

    // @step When I dispatch add-dependency via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-002' dependsOn='AUTH-001'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-dependency".to_string(),
        args_json: r#"{"workUnitId":"AUTH-002","dependsOn":"AUTH-001"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `fspec add-dependency AUTH-002 AUTH-001` afterwards exits 1 with 'Dependency already exists'
    let (code, _stdout, stderr) = run_add_dep(ws.path(), &["AUTH-002", "AUTH-001"]);
    assert_eq!(
        code, 1,
        "second invocation should fail as duplicate; stderr={stderr}"
    );
    assert!(
        stderr.contains("Dependency already exists"),
        "got stderr: {stderr}"
    );

    // @step And the CLI bridge module codelet/fspec/src/add_dependency.rs contains NO inline edge-add, status guard, cycle, or file-write logic — its only computation is shorthand resolution + conflict pre-check + JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_dependency.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_dependency.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge readable");
    let stripped = common::strip_comments(&bridge_src);
    for forbidden in [
        "detect_cycle",
        "detectCircularDependency",
        "ensure_work_units_file",
        "write_json_atomic",
        "blockedReason",
        "iso8601_now",
        "Circular dependency detected",
        "Dependency already exists",
        "does not exist",
    ] {
        assert!(
            !stripped.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{stripped}"
        );
    }
}

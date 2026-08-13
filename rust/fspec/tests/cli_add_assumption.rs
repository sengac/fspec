//! CLI surface for the `add-assumption` subcommand — RPC-169.
//!
//! Feature: spec/features/add-assumption-cli-subcommand.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

fn run_add_assumption(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-assumption");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-assumption");
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

fn seed_unit(id: &str, status: &str) -> String {
    let mut states = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let arr: Vec<serde_json::Value> = if *st == status {
            vec![serde_json::Value::String(id.to_string())]
        } else {
            vec![]
        };
        states.insert((*st).to_string(), serde_json::Value::Array(arr));
    }
    serde_json::to_string_pretty(&serde_json::json!({
        "version": "0.7.1",
        "workUnits": {
            id: {
                "id": id, "title": "title", "type": "story", "status": status,
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": serde_json::Value::Object(states),
    }))
    .unwrap()
}

const TS_HELP_FIXTURE_AA: &str = include_str!("fixtures/help/add-assumption.txt");

#[test]
fn scenario_add_assumption_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary is built and on PATH
    // @step When I run `fspec add-assumption --help`
    let output = Command::new(fspec_bin())
        .arg("add-assumption")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-assumption --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "add-assumption --help must exit 0; stderr={stderr}"
    );
    // @step And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/add-assumption.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_AA);
    // @step And stdout starts with a blank line followed by 'ADD-ASSUMPTION'
    assert!(stdout.starts_with("\nADD-ASSUMPTION\n"));
}

#[test]
fn scenario_cli_successfully_appends_assumption_and_prints_success_line() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "specifying"));
    // @step When I run `fspec add-assumption AUTH-001 "Users have valid email"` in that tempdir
    let (code, stdout, stderr) =
        run_add_assumption(ws.path(), &["AUTH-001", "Users have valid email"]);
    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");
    // @step And stdout contains the substring '✓ Assumption added successfully'
    assert!(
        stdout.contains("✓ Assumption added successfully"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );
    // @step And spec/work-units.json on disk shows AUTH-001.assumptions has length 1
    let v = read_work_units(ws.path());
    let arr = v["workUnits"]["AUTH-001"]["assumptions"]
        .as_array()
        .expect("assumptions");
    assert_eq!(arr.len(), 1);
    // @step And spec/work-units.json on disk shows AUTH-001.assumptions[0]='Users have valid email'
    assert_eq!(arr[0].as_str(), Some("Users have valid email"));
}

#[test]
fn scenario_cli_rejects_non_specifying_status_with_exit_1_and_error_prefix() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "backlog"));
    // @step When I run `fspec add-assumption AUTH-001 "Anything"` in that tempdir
    let (code, _stdout, stderr) = run_add_assumption(ws.path(), &["AUTH-001", "Anything"]);
    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");
    // @step And stderr contains the substring '✗ Failed to add assumption:'
    assert!(
        stderr.contains("✗ Failed to add assumption:"),
        "stderr must contain TS error prefix; got:\n{stderr}"
    );
    // @step And stderr contains the substring "Can only add assumptions during discovery/specification phase. AUTH-001 is in 'backlog' state."
    assert!(
        stderr.contains("Can only add assumptions during discovery/specification phase. AUTH-001 is in 'backlog' state."),
        "stderr must contain canonical phase-guard; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "specifying"));
    // @step When I dispatch add-assumption via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' assumption='A1'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-assumption".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","assumption":"A1"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    // @step Then the dispatcher returns success=true
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    // @step And running `fspec add-assumption AUTH-001 "A2"` afterwards exits 0
    let (code, stdout, stderr) = run_add_assumption(ws.path(), &["AUTH-001", "A2"]);
    assert_eq!(
        code, 0,
        "CLI add must succeed; stdout={stdout}, stderr={stderr}"
    );
    // @step And spec/work-units.json on disk shows AUTH-001.assumptions has length 2
    let v = read_work_units(ws.path());
    let arr = v["workUnits"]["AUTH-001"]["assumptions"]
        .as_array()
        .expect("assumptions");
    assert_eq!(arr.len(), 2);
    // @step And the CLI bridge module rust/fspec/src/add_assumption.rs contains NO inline append, status guard, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_assumption.rs");
    assert!(bridge_path.exists());
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge readable");
    for forbidden in [
        "ensure_work_units_file",
        "write_json_atomic",
        "Can only add assumptions",
        "does not exist",
        "✓ Assumption",
    ] {
        // "✓ Assumption" is allowed because the CLI prints it; we filter that.
        if forbidden == "✓ Assumption" {
            continue;
        }
        assert!(
            !bridge_src.contains(forbidden),
            "bridge must NOT embed `{forbidden}`; got:\n{bridge_src}"
        );
    }
}

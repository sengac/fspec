//! CLI surface for the `add-command` subcommand on the standalone fspec
//! Rust binary — RPC-174.
//!
//! Feature: spec/features/add-command-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file
//! above; @step comments mirror the Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_add_command(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-command");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-command");
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
                "id": id,
                "title": "title",
                "type": "story",
                "status": status,
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": serde_json::Value::Object(states),
    }))
    .unwrap()
}

const TS_HELP_FIXTURE_CMD: &str = include_str!("fixtures/help/add-command.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_add_command_help_matches_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec add-command --help`
    let output = Command::new(fspec_bin())
        .arg("add-command")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-command --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "add-command --help must exit 0; stderr={stderr}");

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-command.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_CMD);

    // @step And stdout starts with a blank line followed by 'ADD-COMMAND'
    assert!(stdout.starts_with("\nADD-COMMAND\n"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI successfully appends a command and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_successfully_appends_command_and_prints_success_line() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I run `fspec add-command AUTH-001 "PlaceOrder" --actor "Customer"` in that tempdir
    let (code, stdout, stderr) = run_add_command(
        ws.path(),
        &["AUTH-001", "PlaceOrder", "--actor", "Customer"],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Added command "PlaceOrder" to AUTH-001 (ID: 0)'
    assert!(
        stdout.contains("✓ Added command \"PlaceOrder\" to AUTH-001 (ID: 0)"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 1
    let v = read_work_units(ws.path());
    let items = v["workUnits"]["AUTH-001"]["eventStorm"]["items"]
        .as_array()
        .expect("items array");
    assert_eq!(items.len(), 1);

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].text='PlaceOrder'
    assert_eq!(items[0]["text"].as_str(), Some("PlaceOrder"));

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].actor='Customer'
    assert_eq!(items[0]["actor"].as_str(), Some("Customer"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects a blocked work unit with exit 1 and the TS-parity error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_blocked_work_unit_with_exit_1_and_error_prefix() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=blocked
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "blocked"));
    let pre_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();

    // @step When I run `fspec add-command AUTH-001 "Anything"` in that tempdir
    let (code, _stdout, stderr) = run_add_command(ws.path(), &["AUTH-001", "Anything"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to add command:'
    assert!(
        stderr.contains("✗ Failed to add command:"),
        "stderr must contain TS error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Cannot add Event Storm items to work unit in blocked state"
    assert!(
        stderr.contains("Cannot add Event Storm items to work unit in blocked state"),
        "stderr must contain canonical state-guard message; got:\n{stderr}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let post_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();
    assert_eq!(pre_bytes, post_bytes);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I dispatch add-command via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' text='C1'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-command".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","text":"C1"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `fspec add-command AUTH-001 "C2"` afterwards exits 0
    let (code, stdout, stderr) = run_add_command(ws.path(), &["AUTH-001", "C2"]);
    assert_eq!(
        code, 0,
        "CLI add must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 2
    let v = read_work_units(ws.path());
    let items = v["workUnits"]["AUTH-001"]["eventStorm"]["items"]
        .as_array()
        .expect("items array");
    assert_eq!(items.len(), 2);

    // @step And the CLI bridge module codelet/fspec/src/add_command.rs contains NO inline item construction, status guard, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_command.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_command.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    // The bridge IS allowed to render the success line (TS-parity: the success line lives in the
    // Commander.js action callback, NOT in the addCommand() function which returns
    // {success, commandId}). Every literal below would constitute domain-logic duplication.
    for forbidden in [
        "process_modeling",
        "nextItemId",
        "write_json_atomic",
        "Cannot add Event Storm items",
        "not found. Run fspec init",
        "\"blue\"",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

//! CLI surface for the `add-hotspot` subcommand on the standalone fspec
//! Rust binary — RPC-185.
//!
//! Feature: spec/features/add-hotspot-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_add_hotspot(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-hotspot");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-hotspot");
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
    for st in &["backlog", "specifying", "testing", "implementing", "validating", "done", "blocked"]
    {
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

const TS_HELP_FIXTURE_AH: &str = include_str!("fixtures/help/add-hotspot.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_add_hotspot_help_matches_ts_fixture() {
    // @step given the fspec Rust binary is built and on PATH

    // @step when I run `fspec add-hotspot --help`
    let output = Command::new(fspec_bin())
        .arg("add-hotspot")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-hotspot --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step then the exit code is 0
    assert_eq!(code, 0, "add-hotspot --help must exit 0; stderr={stderr}");

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-hotspot.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_AH);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI appends a hotspot and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_appends_hotspot_and_prints_success_line() {
    // @step given a project root tempdir with spec/work-units.json containing RPC-185 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("RPC-185", "specifying"));

    // @step when I run `fspec add-hotspot RPC-185 "Unclear retry policy"` in that tempdir
    let (code, stdout, stderr) = run_add_hotspot(ws.path(), &["RPC-185", "Unclear retry policy"]);

    // @step then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Hotspot added to RPC-185 (id: 0)'
    assert!(
        stdout.contains("✓ Hotspot added to RPC-185 (id: 0)"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And spec/work-units.json on disk shows RPC-185 eventStorm items has length 1
    let v = read_work_units(ws.path());
    let items = v["workUnits"]["RPC-185"]["eventStorm"]["items"]
        .as_array()
        .expect("items array");
    assert_eq!(items.len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects a missing work unit with exit 1 and TS-parity error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_missing_work_unit_with_exit_1_and_error_prefix() {
    // @step given a project root tempdir with spec/work-units.json that does not contain "NOPE-1"
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("RPC-185", "specifying"));
    let pre_bytes = fs::read(ws.path().join("spec/work-units.json")).unwrap();

    // @step when I run `fspec add-hotspot NOPE-1 "X"` in that tempdir
    let (code, _stdout, stderr) = run_add_hotspot(ws.path(), &["NOPE-1", "X"]);

    // @step then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to add hotspot:'
    assert!(
        stderr.contains("✗ Failed to add hotspot:"),
        "stderr must contain TS error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Work unit NOPE-1 not found"
    assert!(
        stderr.contains("Work unit NOPE-1 not found"),
        "stderr must contain canonical missing message; got:\n{stderr}"
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
    // @step given a project root tempdir with spec/work-units.json containing RPC-185 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("RPC-185", "specifying"));

    // @step when I dispatch add-hotspot via fspec_core::dispatch::dispatch_command with workUnitId='RPC-185' text='H1'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-hotspot".to_string(),
        args_json: r#"{"workUnitId":"RPC-185","text":"H1"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step then the dispatcher returns success=true
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    // @step And running `fspec add-hotspot RPC-185 "H2"` afterwards exits 0
    let (code, stdout, stderr) = run_add_hotspot(ws.path(), &["RPC-185", "H2"]);
    assert_eq!(code, 0, "CLI add must succeed; stdout={stdout}, stderr={stderr}");

    // @step And spec/work-units.json on disk shows RPC-185 eventStorm items has length 2
    let v = read_work_units(ws.path());
    let items = v["workUnits"]["RPC-185"]["eventStorm"]["items"]
        .as_array()
        .expect("items array");
    assert_eq!(items.len(), 2);

    // @step And the CLI bridge module codelet/fspec/src/add_hotspot.rs contains NO inline item construction, status guard, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_hotspot.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_hotspot.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    // The bridge IS allowed to print the success line (`✓ Hotspot added to ...`), mirroring
    // the TS action callback at src/commands/add-hotspot.ts. Every OTHER literal below would
    // constitute domain-logic duplication and is forbidden.
    for forbidden in [
        "eventStorm",
        "process_modeling",
        "addEventStormItem",
        "add_event_storm_item",
        "write_json_atomic",
        "Cannot add Event Storm items",
        "not found",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

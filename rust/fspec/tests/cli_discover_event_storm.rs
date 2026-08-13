//! CLI surface for the `discover-event-storm` subcommand on the standalone
//! fspec Rust binary — RPC-225.
//!
//! Feature: spec/features/discover-event-storm-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim. Until Phase C wiring
//! (clap variant + main.rs help intercept + dispatcher), these tests fail —
//! the intended red phase.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_discover(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("discover-event-storm");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec discover-event-storm");
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

const TS_HELP_FIXTURE_DES: &str = include_str!("fixtures/help/discover-event-storm.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes discover-event-storm as a subcommand and prints flag-free --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_discover_event_storm_help_matches_ts_fixture() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec discover-event-storm --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("discover-event-storm")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn discover-event-storm --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "discover-event-storm --help must exit 0; stderr={stderr}"
    );

    // @step Then stdout is byte-for-byte identical to the captured TS fixture at rust/fspec/tests/fixtures/help/discover-event-storm.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_DES);

    // @step Then stdout advertises the required positional <work-unit-id> argument
    assert!(
        stdout.contains("work-unit-id") || stdout.contains("workUnitId"),
        "help must advertise the positional argument; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI emits guidance for a work unit in specifying status and exits 0
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_emits_guidance_for_specifying_unit_and_exits_0() {
    // @step Given spec/work-units.json contains AUTH-001 in specifying status in the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I run `./rust/target/release/fspec discover-event-storm AUTH-001` from that directory
    let (code, stdout, stderr) = run_discover(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step Then stdout contains '✓ Event Storm discovery session started for AUTH-001'
    assert!(
        stdout.contains("✓ Event Storm discovery session started for AUTH-001"),
        "stdout must contain the green confirmation; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'EVENT STORM DISCOVERY - AUTH-001'
    assert!(
        stdout.contains("EVENT STORM DISCOVERY - AUTH-001"),
        "stdout must contain the reminder header; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against empty workspace exits 1 with missing-file error
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_empty_workspace_exits_1_missing_file() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec/work-units.json").exists());

    // @step When I run `./rust/target/release/fspec discover-event-storm AUTH-001` from that directory
    let (code, _stdout, stderr) = run_discover(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step Then stderr contains the substring 'Error:'
    // TS uses `output.error('✗ ...')` (not an `Error:` prefix); the binary
    // emits a bare `✗`-prefixed line. Assert byte-parity with the TS CLI.
    assert!(
        stderr.contains("✗"),
        "stderr must contain the ✗ prefix; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'spec/work-units.json not found. Run fspec init first.'
    assert!(
        stderr.contains("spec/work-units.json not found. Run fspec init first."),
        "stderr must contain the missing-file message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when the work unit is not in specifying status
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_1_when_not_specifying() {
    // @step Given spec/work-units.json contains AUTH-001 in backlog status in the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "backlog"));

    // @step When I run `./rust/target/release/fspec discover-event-storm AUTH-001` from that directory
    let (code, _stdout, stderr) = run_discover(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step Then stderr contains the substring 'must be in specifying status (currently: backlog)'
    assert!(
        stderr.contains("must be in specifying status (currently: backlog)"),
        "stderr must contain the status-gate message; got:\n{stderr}"
    );
}

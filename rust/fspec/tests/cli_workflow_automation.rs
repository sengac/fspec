//! CLI surface for the `workflow-automation` subcommand on the standalone
//! fspec Rust binary — RPC-326.
//!
//! Feature: spec/features/workflow-automation-cli-subcommand.feature
//!
//! The TS Commander shell binds correctly (it passes action + workUnitId +
//! options positionally to `workflowAutomation`), so — unlike auto-advance —
//! this is NOT Framing A. The Rust bridge marshals the positional <action> +
//! <work-unit-id> and the --event / --from-state flags into the JSON args
//! shape and delegates to fspec_core::commands::workflow_automation::run. On
//! success the shell prints nothing and exits 0; on error it writes to stderr
//! and exits 1.
//!
//! PHASE B (red): until main.rs registers the clap subcommand AND the
//! intercept arm is added, these tests fail (subcommand not yet wired).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

mod common;

use common::fspec_bin;

fn workspace_with_status(status: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    let spec = dir.path().join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let testing = if status == "testing" {
        "[\"AUTH-001\"]"
    } else {
        "[]"
    };
    let implementing = if status == "implementing" {
        "[\"AUTH-001\"]"
    } else {
        "[]"
    };
    let body = format!(
        r#"{{
  "version": "0.7.1",
  "meta": {{ "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" }},
  "workUnits": {{
    "AUTH-001": {{
      "id": "AUTH-001",
      "title": "Login",
      "status": "{status}",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }}
  }},
  "states": {{
    "backlog": [], "specifying": [], "testing": {testing},
    "implementing": {implementing}, "validating": [], "done": [], "blocked": []
  }}
}}"#
    );
    fs::write(spec.join("work-units.json"), body).expect("write work-units.json");
    dir
}

fn run_workflow_automation(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("workflow-automation");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec workflow-automation");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn read_stored(cwd: &Path) -> Value {
    let raw =
        fs::read_to_string(cwd.join("spec").join("work-units.json")).expect("read work-units.json");
    serde_json::from_str(&raw).expect("stored work-units.json is valid JSON")
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shell record-iteration increments the counter and exits 0
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_shell_record_iteration_increments_counter_and_exits_0() {
    // @step Given a working directory whose spec/work-units.json contains AUTH-001
    let ws = workspace_with_status("implementing");

    // @step When I run `fspec workflow-automation record-iteration AUTH-001` from that directory
    let (code, stdout, stderr) =
        run_workflow_automation(ws.path(), &["record-iteration", "AUTH-001"]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "must exit 0; stdout={stdout}, stderr={stderr}");

    // @step And the persisted AUTH-001 has metrics.iterations equal to 1
    let stored = read_stored(ws.path());
    assert_eq!(
        stored["workUnits"]["AUTH-001"]["metrics"]["iterations"].as_u64(),
        Some(1)
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shell auto-advance advances a testing unit and exits 0
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_shell_auto_advance_advances_testing_unit_and_exits_0() {
    // @step Given a working directory whose spec/work-units.json contains AUTH-001 with status 'testing'
    let ws = workspace_with_status("testing");

    // @step When I run `fspec workflow-automation auto-advance AUTH-001 --event tests-pass --from-state testing` from that directory
    let (code, stdout, stderr) = run_workflow_automation(
        ws.path(),
        &[
            "auto-advance",
            "AUTH-001",
            "--event",
            "tests-pass",
            "--from-state",
            "testing",
        ],
    );

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "must exit 0; stdout={stdout}, stderr={stderr}");

    // @step And the persisted AUTH-001 status is 'implementing'
    let stored = read_stored(ws.path());
    assert_eq!(
        stored["workUnits"]["AUTH-001"]["status"].as_str(),
        Some("implementing")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shell command on a missing work unit exits 1 with an error
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_shell_missing_work_unit_exits_1_with_error() {
    // @step Given a working directory whose spec/work-units.json contains no work unit MISSING-001
    let ws = workspace_with_status("implementing");

    // @step When I run `fspec workflow-automation record-iteration MISSING-001` from that directory
    let (code, stdout, stderr) =
        run_workflow_automation(ws.path(), &["record-iteration", "MISSING-001"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains "Work unit 'MISSING-001' does not exist"
    assert!(
        stderr.contains("Work unit 'MISSING-001' does not exist"),
        "stderr must contain \"Work unit 'MISSING-001' does not exist\"; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: workflow-automation --help is byte-for-byte identical to the TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/workflow-automation.txt");

#[test]
fn scenario_workflow_automation_help_matches_ts_format_command_help_reference() {
    // @step Given the fspec Rust binary has been compiled

    // @step When I run `fspec workflow-automation --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("workflow-automation")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec workflow-automation --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "workflow-automation --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/workflow-automation.txt
    assert_eq!(
        stdout, TS_HELP_FIXTURE,
        "workflow-automation --help must match the TS fixture byte-for-byte; stderr={stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI bridge delegates to the same fspec_core function as the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_bridge_delegates_to_same_fspec_core_function() {
    // @step Given the CLI bridge module rust/fspec/src/workflow_automation.rs
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/workflow_automation.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/workflow_automation.rs must exist as the CLI bridge module; missing: {}",
        bridge_path.display()
    );

    // @step When I inspect its source
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");

    // @step Then it contains no inline action-dispatch, transition, or work-units mutation logic
    // @step And its only computation is JSON arg marshalling before delegating to fspec_core::commands::workflow_automation::run
    for forbidden in [
        "write_json_atomic",
        "Invalid action",
        "Invalid transition",
        "stateHistory",
        "scenariosFound",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
    assert!(
        bridge_src.contains("workflow_automation::run"),
        "bridge must delegate to fspec_core::commands::workflow_automation::run; got:\n{bridge_src}"
    );
}

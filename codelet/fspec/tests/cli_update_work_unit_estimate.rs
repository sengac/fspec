//! CLI surface for the `update-work-unit-estimate` subcommand on the standalone
//! fspec Rust binary — RPC-318.
//!
//! Feature: spec/features/update-work-unit-estimate-cli-subcommand.feature
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

fn run_estimate(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("update-work-unit-estimate");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec update-work-unit-estimate");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units(cwd: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec/work-units.json")).expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn one_task(id: &str) -> String {
    format!(
        r#"{{
  "version": "0.7.1",
  "workUnits": {{
    "{id}": {{
      "id": "{id}", "title": "A task", "type": "task", "status": "specifying",
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
    }}
  }},
  "states": {{
    "backlog": [], "specifying": ["{id}"], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }}
}}"#
    )
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/update-work-unit-estimate.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes update-work-unit-estimate with two positional args in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_estimate_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec update-work-unit-estimate --help`
    let output = Command::new(fspec_bin())
        .arg("update-work-unit-estimate")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn update-work-unit-estimate --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "update-work-unit-estimate --help must exit 0; stderr={stderr}"
    );

    // @step And stdout describes the update-work-unit-estimate subcommand
    assert!(
        stdout.contains("update-work-unit-estimate")
            || stdout.contains("UPDATE-WORK-UNIT-ESTIMATE"),
        "help must describe update-work-unit-estimate; got:\n{stdout}"
    );

    // @step And stdout mentions the `<id>` argument
    assert!(
        stdout.contains("<id>") || stdout.contains("id"),
        "help must mention id; got:\n{stdout}"
    );

    // @step And stdout mentions the `<points>` argument
    assert!(
        stdout.contains("points"),
        "help must mention points; got:\n{stdout}"
    );

    // @step And the --help output is byte-for-byte identical to the captured TS reference fixture
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI sets a task estimate and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_sets_task_estimate_and_prints_success() {
    // @step Given spec/work-units.json contains work unit 'TASK-001' of type 'task'
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &one_task("TASK-001"));

    // @step When I run `./codelet/target/release/fspec update-work-unit-estimate TASK-001 3`
    let (code, stdout, stderr) = run_estimate(ws.path(), &["TASK-001", "3"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Work unit TASK-001 estimate set to 3'
    assert!(
        stdout.lines().any(|l| l == "✓ Work unit TASK-001 estimate set to 3"),
        "missing success line; got:\n{stdout}"
    );

    // @step And spec/work-units.json work unit 'TASK-001' has estimate 3
    let data = read_work_units(ws.path());
    assert_eq!(data["workUnits"]["TASK-001"]["estimate"].as_i64(), Some(3));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports an invalid estimate on stderr
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_invalid_estimate_on_stderr() {
    // @step Given spec/work-units.json contains work unit 'TASK-001' of type 'task'
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &one_task("TASK-001"));

    // @step When I run `./codelet/target/release/fspec update-work-unit-estimate TASK-001 7`
    let (code, stdout, stderr) = run_estimate(ws.path(), &["TASK-001", "7"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring 'Invalid estimate: 7. Must be one of: 1,2,3,5,8,13,21'
    assert!(
        stderr.contains("Invalid estimate: 7. Must be one of: 1,2,3,5,8,13,21"),
        "stderr must mention invalid estimate; got:\n{stderr}"
    );
}

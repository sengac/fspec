//! CLI surface for the `update-work-unit` subcommand on the standalone fspec
//! Rust binary — RPC-317.
//!
//! Feature: spec/features/update-work-unit-cli-subcommand.feature
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

fn run_update_work_unit(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("update-work-unit");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec update-work-unit");
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

fn one_unit(id: &str, title: &str) -> String {
    format!(
        r#"{{
  "version": "0.7.1",
  "workUnits": {{
    "{id}": {{
      "id": "{id}", "title": "{title}", "status": "backlog",
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
    }}
  }},
  "states": {{
    "backlog": ["{id}"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }}
}}"#
    )
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/update-work-unit.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes update-work-unit with positional arg and metadata flags in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_update_work_unit_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec update-work-unit --help`
    let output = Command::new(fspec_bin())
        .arg("update-work-unit")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn update-work-unit --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "update-work-unit --help must exit 0; stderr={stderr}"
    );

    // @step And stdout describes the update-work-unit subcommand
    assert!(
        stdout.contains("update-work-unit") || stdout.contains("UPDATE-WORK-UNIT"),
        "help must describe update-work-unit; got:\n{stdout}"
    );

    // @step And stdout mentions the `<workUnitId>` argument
    assert!(
        stdout.contains("workUnitId"),
        "help must mention workUnitId; got:\n{stdout}"
    );

    // @step And stdout advertises the `--title` flag (or its `-t` short form)
    assert!(
        stdout.contains("--title") || stdout.contains("-t"),
        "help must advertise --title; got:\n{stdout}"
    );

    // @step And stdout advertises the `--description` flag (or its `-d` short form)
    assert!(
        stdout.contains("--description") || stdout.contains("-d"),
        "help must advertise --description; got:\n{stdout}"
    );

    // @step And stdout advertises the `--epic` flag (or its `-e` short form)
    assert!(
        stdout.contains("--epic") || stdout.contains("-e"),
        "help must advertise --epic; got:\n{stdout}"
    );

    // @step And stdout advertises the `--parent` flag (or its `-p` short form)
    assert!(
        stdout.contains("--parent") || stdout.contains("-p"),
        "help must advertise --parent; got:\n{stdout}"
    );

    // @step And the --help output is byte-for-byte identical to the captured TS reference fixture
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI updates a work unit title and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_updates_title_and_prints_success() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' with title 'Login'
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &one_unit("AUTH-001", "Login"));

    // @step When I run `./rust/target/release/fspec update-work-unit AUTH-001 --title New`
    let (code, stdout, stderr) = run_update_work_unit(ws.path(), &["AUTH-001", "--title", "New"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Work unit AUTH-001 updated successfully'
    assert!(
        stdout
            .lines()
            .any(|l| l == "✓ Work unit AUTH-001 updated successfully"),
        "missing success line; got:\n{stdout}"
    );

    // @step And spec/work-units.json work unit 'AUTH-001' has title 'New'
    let data = read_work_units(ws.path());
    assert_eq!(data["workUnits"]["AUTH-001"]["title"].as_str(), Some("New"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports failure for a missing work unit on stderr
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_missing_work_unit_on_stderr() {
    // @step Given an empty working directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec update-work-unit MISSING-999 --title X`
    let (code, stdout, stderr) = run_update_work_unit(ws.path(), &["MISSING-999", "--title", "X"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring "Work unit 'MISSING-999' does not exist"
    assert!(
        stderr.contains("Work unit 'MISSING-999' does not exist"),
        "stderr must mention missing work unit; got:\n{stderr}"
    );
}

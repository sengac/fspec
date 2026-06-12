//! CLI surface for the `export-work-units` subcommand on the standalone fspec
//! Rust binary — RPC-229.
//!
//! Feature: spec/features/export-work-units-cli-subcommand.feature
//!
//! Framing A: the TypeScript shell success log references result.count and
//! result.outputFile which are undefined (the function only returns
//! {success:true}), so the shell prints "Exported undefined work units to
//! undefined". The Rust CLI bridge mirrors this broken behaviour.
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

fn workspace_with_valid_work_units() -> TempDir {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    let spec = dir.path().join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let body = r#"{
  "version": "0.7.1",
  "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Login",
      "status": "implementing",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    },
    "AUTH-002": {
      "id": "AUTH-002",
      "title": "Logout",
      "status": "backlog",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  },
  "states": {
    "backlog": ["AUTH-002"], "specifying": [], "testing": [],
    "implementing": ["AUTH-001"], "validating": [], "done": [], "blocked": []
  }
}"#;
    fs::write(spec.join("work-units.json"), body).expect("write work-units.json");
    dir
}

fn run_export(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("export-work-units");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec export-work-units");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

#[test]
fn scenario_cli_exports_work_units_to_json_and_prints_framing_a_success_line() {
    // @step Given a workspace with a valid spec/work-units.json
    let ws = workspace_with_valid_work_units();

    // @step When I run "fspec export-work-units json out.json" in that workspace
    let (code, stdout, stderr) = run_export(ws.path(), &["json", "out.json"]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "must exit 0; stdout={stdout}, stderr={stderr}");

    // @step And the file "out.json" is written with the exported work units
    let written =
        fs::read_to_string(ws.path().join("out.json")).expect("out.json must be written");
    let arr: Value = serde_json::from_str(&written).expect("out.json is valid JSON");
    let ids: Vec<&str> = arr
        .as_array()
        .expect("exported array")
        .iter()
        .map(|u| u["id"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(ids, vec!["AUTH-001", "AUTH-002"]);

    // @step And stdout contains "Exported undefined work units to undefined"
    assert!(
        stdout.contains("Exported undefined work units to undefined"),
        "stdout must mirror the broken TS success line; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_export_work_units_csv_fails_with_unsupported_format_error() {
    // @step Given a workspace with a valid spec/work-units.json
    let ws = workspace_with_valid_work_units();

    // @step When I run "fspec export-work-units csv out.csv" in that workspace
    let (code, stdout, stderr) = run_export(ws.path(), &["csv", "out.csv"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1 on csv; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains "Failed to export work units: Unsupported format: csv"
    assert!(
        stderr.contains("Failed to export work units: Unsupported format: csv"),
        "stderr must contain the canonical unsupported-format error; got:\n{stderr}"
    );
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/export-work-units.txt");

#[test]
fn scenario_cli_export_work_units_help_matches_ts_format_command_help_reference() {
    // @step When I run "fspec export-work-units --help"
    let output = Command::new(fspec_bin())
        .arg("export-work-units")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec export-work-units --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then stdout is byte-for-byte identical to the captured help fixture
    assert_eq!(
        stdout, TS_HELP_FIXTURE,
        "export-work-units --help must match the TS fixture byte-for-byte; stderr={stderr}"
    );

    // @step And the command exits with code 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");
}

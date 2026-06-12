//! CLI surface for the `record-iteration` subcommand on the standalone fspec
//! Rust binary — RPC-264.
//!
//! Feature: spec/features/record-iteration-cli-subcommand.feature
//!
//! Framing A: the TypeScript shell is broken — the Commander action passes
//! name/start/end and NEVER wires workUnitId, so the function reads an
//! undefined id and ALWAYS fails with "Work unit undefined not found" and exit
//! code 1. The Rust CLI bridge mirrors this broken behaviour verbatim.
//!
//! PHASE B (red): until main.rs registers the clap subcommand AND the
//! intercept arm is added, these tests fail (subcommand not yet wired).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

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
    }
  },
  "states": {
    "backlog": [], "specifying": [], "testing": [],
    "implementing": ["AUTH-001"], "validating": [], "done": [], "blocked": []
  }
}"#;
    fs::write(spec.join("work-units.json"), body).expect("write work-units.json");
    dir
}

fn run_record_iteration(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("record-iteration");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec record-iteration");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

#[test]
fn scenario_cli_record_iteration_always_fails_per_framing_a() {
    // @step Given a workspace with a valid spec/work-units.json
    let ws = workspace_with_valid_work_units();

    // @step When I run "fspec record-iteration Sprint-1" in that workspace
    let (code, stdout, stderr) = run_record_iteration(ws.path(), &["Sprint-1"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "record-iteration must exit 1 (Framing A); stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains "Failed to record iteration"
    assert!(
        stderr.contains("Failed to record iteration"),
        "stderr must contain 'Failed to record iteration'; got:\n{stderr}"
    );

    // @step And stderr contains "Work unit undefined not found"
    assert!(
        stderr.contains("Work unit undefined not found"),
        "stderr must contain 'Work unit undefined not found'; got:\n{stderr}"
    );
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/record-iteration.txt");

#[test]
fn scenario_cli_record_iteration_help_matches_ts_format_command_help_reference() {
    // @step When I run "fspec record-iteration --help"
    let output = Command::new(fspec_bin())
        .arg("record-iteration")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec record-iteration --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then stdout is byte-for-byte identical to the captured help fixture
    assert_eq!(
        stdout, TS_HELP_FIXTURE,
        "record-iteration --help must match the TS fixture byte-for-byte; stderr={stderr}"
    );

    // @step And the command exits with code 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");
}

//! CLI surface for the `export-example-map` subcommand on the standalone
//! fspec Rust binary — RPC-228.
//!
//! Feature: spec/features/export-example-map-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand is wired (Phase C), these tests
//! exercise the binary and the success-path assertions fail (clap reports
//! an unrecognized subcommand). Once the subcommand is wired, the
//! green-phase assertions take over.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_cmd(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("export-example-map");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec export-example-map");
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

fn store_with_auth_001() -> String {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Login feature",
      "status": "backlog",
      "rules": [{ "id": 0, "text": "r1", "deleted": false, "createdAt": "x" }],
      "examples": [{ "id": 0, "text": "e1", "deleted": false, "createdAt": "x" }],
      "questions": [{ "id": 0, "text": "q1", "deleted": false, "selected": false, "createdAt": "x" }],
      "assumptions": ["a1"],
      "createdAt": "x",
      "updatedAt": "x"
    }
  },
  "states": {
    "backlog": ["AUTH-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#
    .to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_export_example_map_writes_file_and_prints_success() {
    // @step Given a workspace whose spec/work-units.json contains AUTH-001 with example mapping data
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &store_with_auth_001());

    // @step When I run `fspec export-example-map AUTH-001 emap.json`
    let (code, stdout, stderr) = run_cmd(ws.path(), &["AUTH-001", "emap.json"]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stdout={stdout}, stderr={stderr}");

    // @step And stdout contains "✓ Exported to emap.json"
    assert!(
        stdout.contains("✓ Exported to emap.json"),
        "stdout must contain success message; got:\n{stdout}"
    );

    // @step And emap.json contains the exported example mapping JSON
    let written = fs::read_to_string(ws.path().join("emap.json")).expect("read emap.json");
    let parsed: serde_json::Value = serde_json::from_str(&written).expect("emap.json is JSON");
    assert_eq!(parsed["workUnitId"].as_str(), Some("AUTH-001"));
    assert_eq!(parsed["rules"][0]["text"].as_str(), Some("r1"));
}

#[test]
fn scenario_cli_export_example_map_fails_for_unknown_work_unit() {
    // @step Given a workspace whose spec/work-units.json does not contain NOPE-999
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &store_with_auth_001());

    // @step When I run `fspec export-example-map NOPE-999 out.json`
    let (code, stdout, stderr) = run_cmd(ws.path(), &["NOPE-999", "out.json"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains "✗ Failed to export example map: Work unit 'NOPE-999' does not exist"
    assert!(
        stderr.contains("✗ Failed to export example map: Work unit 'NOPE-999' does not exist"),
        "stderr must contain the canonical failure message; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_export_example_map_requires_file_argument() {
    // @step Given an empty workspace
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec export-example-map AUTH-001`
    let (code, _stdout, stderr) = run_cmd(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with a non-zero code
    assert_ne!(code, 0, "missing argument must produce a non-zero exit");

    // @step And stderr reports a missing required argument
    let lc = stderr.to_lowercase();
    assert!(
        lc.contains("required") || lc.contains("missing") || lc.contains("usage"),
        "stderr must report a missing required argument; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_export_example_map_help_matches_fixture() {
    // @step Given an empty workspace
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec export-example-map --help`
    let output = Command::new(fspec_bin())
        .arg("export-example-map")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .current_dir(ws.path())
        .output()
        .expect("spawn fspec export-example-map --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then stdout matches the captured export-example-map help fixture
    assert_eq!(code, 0, "help must exit 0; stderr={stderr}");
    let fixture = include_str!("fixtures/help/export-example-map.txt");
    assert_eq!(stdout, fixture);
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a workspace whose spec/work-units.json contains AUTH-001 with example mapping data
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &store_with_auth_001());

    // @step When I export AUTH-001 via the CLI and via the dispatcher into separate files
    let (cli_code, _o, _e) = run_cmd(ws.path(), &["AUTH-001", "cli.json"]);
    assert_eq!(cli_code, 0, "CLI export must succeed");

    let req = codelet_fspec_core::DispatchRequest {
        command: "export-example-map".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","file":"disp.json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    // @step Then both files have identical content
    let cli_content = fs::read_to_string(ws.path().join("cli.json")).expect("read cli.json");
    let disp_content = fs::read_to_string(ws.path().join("disp.json")).expect("read disp.json");
    assert_eq!(
        cli_content, disp_content,
        "CLI and dispatcher must produce identical file content"
    );
}

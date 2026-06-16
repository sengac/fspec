//! CLI surface for the `generate-summary-report` subcommand on the standalone
//! fspec Rust binary — RPC-235.
//!
//! Feature: spec/features/generate-summary-report-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand is wired (Phase C), these tests
//! exercise the binary and the success-path assertions fail (clap reports an
//! unrecognized subcommand). Once the subcommand is wired, the green-phase
//! assertions take over.

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
    cmd.arg("generate-summary-report");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec generate-summary-report");
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

fn store_with_units() -> String {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "A-1": { "id": "A-1", "title": "t1", "status": "done", "estimate": 3, "createdAt": "x", "updatedAt": "x" },
    "A-2": { "id": "A-2", "title": "t2", "status": "done", "estimate": 5, "createdAt": "x", "updatedAt": "x" },
    "A-3": { "id": "A-3", "title": "t3", "status": "backlog", "estimate": 2, "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": ["A-3"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": ["A-1", "A-2"], "blocked": []
  }
}"#
    .to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_generate_summary_report_writes_report_and_prints_success() {
    // @step Given a workspace whose spec/work-units.json contains a few work units
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &store_with_units());

    // @step When I run `fspec generate-summary-report --output report.md`
    let (code, stdout, stderr) = run_cmd(ws.path(), &["--output", "report.md"]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stdout={stdout}, stderr={stderr}");

    // @step And stdout contains "✓ Report generated: report.md"
    assert!(
        stdout.contains("✓ Report generated: report.md"),
        "stdout must contain success message; got:\n{stdout}"
    );

    // @step And report.md contains the rendered summary report
    let written = fs::read_to_string(ws.path().join("report.md")).expect("read report.md");
    assert!(
        written.contains("# Project Summary Report"),
        "report.md must contain the rendered report; got:\n{written}"
    );
}

#[test]
fn scenario_cli_generate_summary_report_format_json_writes_json_report() {
    // @step Given a workspace whose spec/work-units.json contains a few work units
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &store_with_units());

    // @step When I run `fspec generate-summary-report --format json`
    let (code, stdout, stderr) = run_cmd(ws.path(), &["--format", "json"]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stdout={stdout}, stderr={stderr}");

    // @step And spec/summary-report.json contains the pretty-printed report JSON
    let written =
        fs::read_to_string(ws.path().join("spec/summary-report.json")).expect("read report json");
    let parsed: serde_json::Value = serde_json::from_str(&written).expect("report is JSON");
    assert_eq!(parsed["totalWorkUnits"].as_i64(), Some(3));
    assert!(
        written.contains("\n  \"totalWorkUnits\""),
        "report must be pretty-printed; got:\n{written}"
    );
}

#[test]
fn scenario_cli_generate_summary_report_fails_when_work_units_file_missing() {
    // @step Given an empty workspace with no spec/work-units.json
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec generate-summary-report`
    let (code, stdout, stderr) = run_cmd(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains "✗ Failed to generate report:"
    assert!(
        stderr.contains("✗ Failed to generate report:"),
        "stderr must contain the failure message; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_generate_summary_report_help_matches_fixture() {
    // @step Given an empty workspace
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec generate-summary-report --help`
    let output = Command::new(fspec_bin())
        .arg("generate-summary-report")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .current_dir(ws.path())
        .output()
        .expect("spawn fspec generate-summary-report --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then stdout matches the captured generate-summary-report help fixture
    assert_eq!(code, 0, "help must exit 0; stderr={stderr}");
    let fixture = include_str!("fixtures/help/generate-summary-report.txt");
    assert_eq!(stdout, fixture);
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a workspace whose spec/work-units.json contains a few work units
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &store_with_units());

    // @step When I generate a report via the CLI and via the dispatcher into separate files
    let (cli_code, _o, _e) = run_cmd(ws.path(), &["--format", "json", "--output", "cli.json"]);
    assert_eq!(cli_code, 0, "CLI generate must succeed");

    let req = codelet_fspec_core::DispatchRequest {
        command: "generate-summary-report".to_string(),
        args_json: r#"{"format":"json","output":"disp.json"}"#.to_string(),
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

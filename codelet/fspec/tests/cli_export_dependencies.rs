//! CLI surface for the `export-dependencies` subcommand on the standalone
//! fspec Rust binary — RPC-227.
//!
//! Feature: spec/features/export-dependencies-cli-subcommand.feature
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
    cmd.arg("export-dependencies");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec export-dependencies");
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

fn store_with_dependencies() -> String {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Login feature",
      "status": "backlog",
      "blocks": ["AUTH-002"],
      "dependsOn": ["AUTH-003"],
      "createdAt": "x",
      "updatedAt": "x"
    },
    "AUTH-002": {
      "id": "AUTH-002",
      "title": "Logout feature",
      "status": "done",
      "createdAt": "x",
      "updatedAt": "x"
    }
  },
  "states": {
    "backlog": ["AUTH-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": ["AUTH-002"], "blocked": []
  }
}"#
    .to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_export_dependencies_mermaid_writes_file_and_prints_success() {
    // @step Given a workspace whose spec/work-units.json contains work units with dependencies
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &store_with_dependencies());

    // @step When I run `fspec export-dependencies mermaid deps.mmd`
    let (code, stdout, stderr) = run_cmd(ws.path(), &["mermaid", "deps.mmd"]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stdout={stdout}, stderr={stderr}");

    // @step And stdout contains "✓ Dependencies exported to deps.mmd"
    assert!(
        stdout.contains("✓ Dependencies exported to deps.mmd"),
        "stdout must contain success message; got:\n{stdout}"
    );

    // @step And deps.mmd contains a graph TB diagram
    let written = fs::read_to_string(ws.path().join("deps.mmd")).expect("read deps.mmd");
    assert!(written.starts_with("graph TB"), "got:\n{written}");
}

#[test]
fn scenario_cli_export_dependencies_json_writes_dependency_map() {
    // @step Given a workspace whose spec/work-units.json contains work units with dependencies
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &store_with_dependencies());

    // @step When I run `fspec export-dependencies json deps.json`
    let (code, stdout, stderr) = run_cmd(ws.path(), &["json", "deps.json"]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stdout={stdout}, stderr={stderr}");

    // @step And deps.json contains the dependency map keyed by work unit id
    let written = fs::read_to_string(ws.path().join("deps.json")).expect("read deps.json");
    let parsed: serde_json::Value = serde_json::from_str(&written).expect("deps.json is JSON");
    assert!(parsed["AUTH-001"]["blocks"].is_array());
    assert_eq!(parsed["AUTH-001"]["blocks"][0].as_str(), Some("AUTH-002"));
    assert!(parsed["AUTH-002"]["blocks"].is_array());
}

#[test]
fn scenario_cli_export_dependencies_requires_output_argument() {
    // @step Given an empty workspace
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec export-dependencies mermaid`
    let (code, _stdout, stderr) = run_cmd(ws.path(), &["mermaid"]);

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
fn scenario_cli_export_dependencies_help_matches_fixture() {
    // @step Given an empty workspace
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec export-dependencies --help`
    let output = Command::new(fspec_bin())
        .arg("export-dependencies")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .current_dir(ws.path())
        .output()
        .expect("spawn fspec export-dependencies --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then stdout matches the captured export-dependencies help fixture
    assert_eq!(code, 0, "help must exit 0; stderr={stderr}");
    let fixture = include_str!("fixtures/help/export-dependencies.txt");
    assert_eq!(stdout, fixture);
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a workspace whose spec/work-units.json contains work units with dependencies
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &store_with_dependencies());

    // @step When I export the dependencies via the CLI and via the dispatcher into separate files
    let (cli_code, _o, _e) = run_cmd(ws.path(), &["json", "cli.json"]);
    assert_eq!(cli_code, 0, "CLI export must succeed");

    let req = codelet_fspec_core::DispatchRequest {
        command: "export-dependencies".to_string(),
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

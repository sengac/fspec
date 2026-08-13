//! CLI surface for the `query-work-units` subcommand on the standalone fspec
//! Rust binary — RPC-263.
//!
//! Feature: spec/features/query-work-units-cli-subcommand.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

mod common;

use common::fspec_bin;

fn empty_workspace() -> TempDir {
    tempfile::tempdir().expect("create empty workspace tempdir")
}

fn workspace_with_work_units_json(body: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("create workspace tempdir");
    let spec = dir.path().join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), body).expect("write work-units.json");
    dir
}

fn store_with_two_units() -> String {
    r#"{
  "version": "0.7.1",
  "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Login feature",
      "status": "implementing",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    },
    "AUTH-002": {
      "id": "AUTH-002",
      "title": "Logout feature",
      "status": "backlog",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#
    .to_string()
}

fn store_with_tags() -> String {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Login",
      "status": "backlog",
      "tags": ["@cli"],
      "createdAt": "x",
      "updatedAt": "x"
    },
    "AUTH-002": {
      "id": "AUTH-002",
      "title": "Logout",
      "status": "backlog",
      "tags": ["@high"],
      "createdAt": "x",
      "updatedAt": "x"
    }
  }
}"#
    .to_string()
}

fn run_qwu(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("query-work-units");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec query-work-units");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

#[test]
fn scenario_standalone_fspec_binary_exposes_query_work_units_as_a_clap_subcommand() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec query-work-units --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("query-work-units")
        .arg("--help")
        .output()
        .expect("spawn fspec query-work-units --help");

    // @step Then the command exits 0
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step Then stdout lists the flags --status, --prefix, --epic, --type, --tag, and --format
    let help = if stdout.is_empty() {
        stderr.clone()
    } else {
        stdout.clone()
    };
    for flag in [
        "--status", "--prefix", "--epic", "--type", "--tag", "--format",
    ] {
        assert!(
            help.contains(flag),
            "help output must list {flag}; got:\n{help}"
        );
    }
}

#[test]
fn scenario_cli_format_json_prints_parseable_json_to_stdout() {
    // @step Given spec/work-units.json contains AUTH-001 (implementing) and AUTH-002 (backlog)
    let ws = workspace_with_work_units_json(&store_with_two_units());

    // @step When I run `./rust/target/release/fspec query-work-units --status=implementing --format=json`
    let (code, stdout, stderr) = run_qwu(ws.path(), &["--status=implementing", "--format=json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stdout={stdout}, stderr={stderr}");

    // @step Then stdout is a parseable JSON object whose workUnits array contains only AUTH-001
    let parsed: Value = serde_json::from_str(stdout.trim()).expect("stdout must be parseable JSON");
    let arr = parsed["workUnits"].as_array().expect("workUnits array");
    let ids: Vec<&str> = arr.iter().map(|e| e["id"].as_str().unwrap_or("")).collect();
    assert_eq!(ids, vec!["AUTH-001"]);

    // @step Then the parsed JSON object contains a top-level `format` field equal to 'json'
    assert_eq!(parsed["format"].as_str(), Some("json"));
}

#[test]
fn scenario_cli_format_text_prints_nothing_to_stdout_per_ts_quirk() {
    // @step Given spec/work-units.json contains AUTH-001 (implementing)
    let ws = workspace_with_work_units_json(&store_with_two_units());

    // @step When I run `./rust/target/release/fspec query-work-units --status=implementing --format=text`
    let (code, stdout, stderr) = run_qwu(ws.path(), &["--status=implementing", "--format=text"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step Then stdout is empty (the TS Commander action does NOT log for non-json formats)
    assert!(
        stdout.is_empty(),
        "stdout MUST be empty for --format=text per TS quirk; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_tag_filter_matches_dispatcher_behavior() {
    // @step Given spec/work-units.json contains AUTH-001 (tags ['@cli']) and AUTH-002 (tags ['@high'])
    let ws = workspace_with_work_units_json(&store_with_tags());

    // @step When I run `./rust/target/release/fspec query-work-units --tag=@cli --format=json`
    let (code, stdout, stderr) = run_qwu(ws.path(), &["--tag=@cli", "--format=json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step Then stdout's workUnits array contains only AUTH-001
    let parsed: Value = serde_json::from_str(stdout.trim()).expect("stdout must be parseable JSON");
    let ids: Vec<&str> = parsed["workUnits"]
        .as_array()
        .expect("workUnits array")
        .iter()
        .map(|e| e["id"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(ids, vec!["AUTH-001"]);
}

#[test]
fn scenario_cli_exits_1_and_writes_to_stderr_when_work_units_json_missing() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = empty_workspace();
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec query-work-units` from that directory
    let (code, stdout, stderr) = run_qwu(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1 on missing work-units.json; stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Failed to query work units:'
    assert!(
        stderr.contains("Failed to query work units:"),
        "stderr must contain canonical prefix; got:\n{stderr}"
    );

    // @step Then spec/work-units.json is NOT auto-created in the directory
    assert!(
        !ws.path().join("spec").join("work-units.json").exists(),
        "spec/work-units.json must NOT be auto-created (unlike list-work-units)"
    );
}

#[test]
fn scenario_cli_exits_1_and_writes_to_stderr_when_work_units_json_is_malformed() {
    // @step Given spec/work-units.json exists in the working directory but contains invalid JSON
    let ws = workspace_with_work_units_json("{ this is not valid json");

    // @step When I run `./rust/target/release/fspec query-work-units` from that directory
    let (code, stdout, stderr) = run_qwu(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1 on malformed JSON; stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Failed to query work units:'
    assert!(
        stderr.contains("Failed to query work units:"),
        "stderr must contain canonical prefix; got:\n{stderr}"
    );
}

#[test]
fn scenario_subcommand_help_excludes_the_global_workspace_flag() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec query-work-units --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("query-work-units")
        .arg("--help")
        .output()
        .expect("spawn fspec query-work-units --help");

    // @step Then the command exits 0
    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step Then stdout does NOT contain the substring '--workspace'
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        !stdout.contains("--workspace"),
        "query-work-units --help must NOT advertise the global --workspace flag; got:\n{stdout}"
    );
}

const TS_HELP_FIXTURE_QWU: &str = include_str!("fixtures/help/query-work-units.txt");

#[test]
fn scenario_query_work_units_help_matches_ts_format_command_help_reference_fixture() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec query-work-units --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("query-work-units")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn query-work-units --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/query-work-units.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_QWU);

    // @step Then stdout starts with a blank line followed by 'QUERY-WORK-UNITS'
    assert!(stdout.starts_with("\nQUERY-WORK-UNITS\n"));
}

#[test]
fn scenario_default_combined_tui_mode_is_preserved_when_no_subcommand() {
    // @step Given the fspec Rust binary has query-work-units registered as a clap subcommand alongside daemon, client, status, and list-work-units

    // @step When I run `./rust/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "must exit 0");
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists query-work-units as an available subcommand
    assert!(
        help.contains("query-work-units"),
        "fspec --help must list query-work-units; got:\n{help}"
    );
}

//! CLI surface for the `validate-foundation-schema` subcommand on the
//! standalone fspec Rust binary — RPC-321.
//!
//! Feature: spec/features/validate-foundation-schema-cli-subcommand.feature
//!
//! PHASE B (TESTING): the clap subcommand is not yet wired into main.rs and
//! the core impl is still a NotYetPorted stub, so these tests are RED until
//! PHASE C + the supervisor's shared-file wiring pass. Each scenario maps 1:1
//! to a Gherkin scenario; @step comments mirror the step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ───────── helpers ─────────

fn run_vfs(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("validate-foundation-schema");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec validate-foundation-schema");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_file(cwd: &Path, rel: &str, body: &str) {
    let path = cwd.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

const VALID_FOUNDATION: &str = r#"{
  "version": "2.0.0",
  "project": { "name": "T", "vision": "v", "projectType": "cli-tool" },
  "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "medium" } },
  "solutionSpace": { "overview": "o", "capabilities": [{ "name": "C", "description": "d" }] }
}"#;

const EMPTY_CAPABILITIES: &str = r#"{
  "version": "2.0.0",
  "project": { "name": "T", "vision": "v", "projectType": "cli-tool" },
  "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "medium" } },
  "solutionSpace": { "overview": "o", "capabilities": [] }
}"#;

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/validate-foundation-schema.txt");

// ───────── scenarios ─────────

#[test]
fn cli_validates_a_valid_foundation_and_exits_0() {
    // Scenario: CLI validates a valid foundation and exits 0

    // @step Given spec/foundation.json contains a schema-valid minimal foundation in the working directory
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/foundation.json", VALID_FOUNDATION);

    // @step When I run `./codelet/target/release/fspec validate-foundation-schema` from that directory
    let (code, stdout, stderr) = run_vfs(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stdout={stdout}, stderr={stderr}");

    // @step Then stdout contains the substring '✓ foundation.json is valid according to the schema'
    assert!(
        stdout.contains("✓ foundation.json is valid according to the schema"),
        "stdout must report success; got:\n{stdout}"
    );
}

#[test]
fn cli_exits_1_and_writes_to_stderr_when_foundation_json_is_missing() {
    // Scenario: CLI exits 1 and writes to stderr when foundation.json is missing

    // @step Given an empty directory with no spec/foundation.json is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec/foundation.json").exists());

    // @step When I run `./codelet/target/release/fspec validate-foundation-schema` from that directory
    let (code, stdout, stderr) = run_vfs(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; stdout={stdout}, stderr={stderr}");

    // @step Then stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "stderr must carry 'Error:'; got:\n{stderr}");

    // @step Then stderr contains the substring 'foundation.json not found in spec/ directory'
    assert!(
        stderr.contains("foundation.json not found in spec/ directory"),
        "stderr must carry the not-found message; got:\n{stderr}"
    );
}

#[test]
fn cli_exits_1_when_foundation_json_violates_the_schema() {
    // Scenario: CLI exits 1 when foundation.json violates the schema

    // @step Given spec/foundation.json has an empty solutionSpace.capabilities array in the working directory
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/foundation.json", EMPTY_CAPABILITIES);

    // @step When I run `./codelet/target/release/fspec validate-foundation-schema` from that directory
    let (code, stdout, stderr) = run_vfs(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; stdout={stdout}, stderr={stderr}");

    // @step Then stderr contains the substring 'Field solutionSpace.capabilities must have at least 1 items (found 0)'
    assert!(
        stderr.contains("Field solutionSpace.capabilities must have at least 1 items (found 0)"),
        "stderr must carry the minItems message; got:\n{stderr}"
    );
}

#[test]
fn clap_exposes_validate_foundation_schema_and_prints_help_byte_identical() {
    // Scenario: Clap exposes validate-foundation-schema as a subcommand and prints help byte-identical to the TS reference

    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec validate-foundation-schema --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("validate-foundation-schema")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn validate-foundation-schema --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "help must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the captured TS formatCommandHelp reference fixture
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step Then stdout does NOT contain the substring '--format'
    assert!(
        !stdout.contains("--format"),
        "flag-less command help must not mention --format; got:\n{stdout}"
    );
}

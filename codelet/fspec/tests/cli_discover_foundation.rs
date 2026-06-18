//! CLI surface for the `discover-foundation` subcommand on the standalone
//! fspec Rust binary — RPC-226.
//!
//! Feature: spec/features/discover-foundation-cli-subcommand.feature
//! (includes the two-front-doors / CLI-bridge-delegation scenario)
//!
//! Each scenario maps 1:1 to a Gherkin scenario; @step comments mirror the
//! Gherkin step text verbatim.
//!
//! RED PHASE: the command is still a stub; these tests FAIL now.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::{json, Value};

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_cmd(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("discover-foundation");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec discover-foundation");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn valid_filled_draft() -> Value {
    json!({
        "version": "2.0.0",
        "project": { "name": "Acme", "vision": "Ship faster", "projectType": "cli-tool" },
        "problemSpace": {
            "primaryProblem": { "title": "Pain", "description": "Real pain", "impact": "high" }
        },
        "solutionSpace": {
            "overview": "A CLI",
            "capabilities": [ { "name": "Cap", "description": "Does things" } ]
        },
        "personas": [ { "name": "Dev", "description": "Builds", "goals": ["Ship"] } ]
    })
}

fn placeholder_draft() -> Value {
    json!({
        "version": "2.0.0",
        "project": {
            "name": "[QUESTION: What is the project name?]",
            "vision": "[QUESTION: What is the one-sentence vision?]",
            "projectType": "[DETECTED: cli-tool]"
        },
        "problemSpace": {
            "primaryProblem": {
                "title": "[QUESTION: What problem does this solve?]",
                "description": "[QUESTION: What problem does this solve?]",
                "impact": "high"
            }
        },
        "solutionSpace": { "overview": "[QUESTION: What can users DO?]", "capabilities": [] },
        "personas": [
            {
                "name": "[QUESTION: Who uses this?]",
                "description": "[QUESTION: Who uses this?]",
                "goals": ["[QUESTION: What are their goals?]"]
            }
        ]
    })
}

fn write_draft(project_root: &Path, value: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json.draft"),
        serde_json::to_string_pretty(value).expect("ser draft"),
    )
    .expect("write draft");
}

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path).expect("read file")
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/discover-foundation.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenarios — discover-foundation-cli-subcommand.feature
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cli_creates_the_draft_and_prints_next_steps_guidance() {
    // @step Given an empty project root tempdir
    let tmp = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec discover-foundation` in that directory
    let (code, stdout, _stderr) = run_cmd(tmp.path(), &[]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stdout={stdout}");

    // @step And stdout contains "✓ Generated spec/foundation.json.draft"
    assert!(stdout.contains("✓ Generated spec/foundation.json.draft"), "stdout={stdout}");

    // @step And stdout contains "Next steps:"
    assert!(stdout.contains("Next steps:"), "stdout={stdout}");

    // @step And stdout contains "1. Use fspec update-foundation commands to fill [QUESTION: ...] placeholders"
    assert!(
        stdout.contains("1. Use fspec update-foundation commands to fill [QUESTION: ...] placeholders"),
        "stdout={stdout}"
    );

    // @step And stdout contains "2. When complete, run: fspec discover-foundation --finalize"
    assert!(
        stdout.contains("2. When complete, run: fspec discover-foundation --finalize"),
        "stdout={stdout}"
    );
}

#[test]
fn cli_fails_when_draft_already_exists_without_force() {
    // @step Given a project root tempdir that already has a spec/foundation.json.draft
    let tmp = tempfile::tempdir().expect("tempdir");
    write_draft(tmp.path(), &placeholder_draft());

    // @step When I run `fspec discover-foundation` in that directory
    let (code, stdout, stderr) = run_cmd(tmp.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout} stderr={stderr}");

    // @step And stderr contains "✗ Failed to create draft"
    assert!(stderr.contains("✗ Failed to create draft"), "stderr={stderr}");

    // @step And stdout contains "ERROR: foundation.json.draft already exists!"
    assert!(stdout.contains("ERROR: foundation.json.draft already exists!"), "stdout={stdout}");
}

#[test]
fn cli_finalize_success_prints_generated_foundation_lines() {
    // @step Given a project root tempdir whose spec/foundation.json.draft is fully filled and schema-valid
    let tmp = tempfile::tempdir().expect("tempdir");
    write_draft(tmp.path(), &valid_filled_draft());

    // @step When I run `fspec discover-foundation --finalize` in that directory
    let (code, stdout, stderr) = run_cmd(tmp.path(), &["--finalize"]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stdout={stdout} stderr={stderr}");

    // @step And stdout contains "✓ Generated spec/foundation.json"
    assert!(stdout.contains("✓ Generated spec/foundation.json"), "stdout={stdout}");

    // @step And stdout contains "✓ Foundation discovered and validated successfully"
    assert!(stdout.contains("✓ Foundation discovered and validated successfully"), "stdout={stdout}");
}

#[test]
fn cli_finalize_failure_on_incomplete_draft_exits_1_with_validation_errors() {
    // @step Given a project root tempdir whose spec/foundation.json.draft still has [QUESTION:] placeholders
    let tmp = tempfile::tempdir().expect("tempdir");
    write_draft(tmp.path(), &placeholder_draft());

    // @step When I run `fspec discover-foundation --finalize` in that directory
    let (code, stdout, stderr) = run_cmd(tmp.path(), &["--finalize"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout} stderr={stderr}");

    // @step And stderr contains "✗ Foundation validation failed"
    assert!(stderr.contains("✗ Foundation validation failed"), "stderr={stderr}");

    // @step And stderr contains "Cannot finalize: draft still has unfilled placeholder fields"
    assert!(
        stderr.contains("Cannot finalize: draft still has unfilled placeholder fields"),
        "stderr={stderr}"
    );
}

#[test]
fn cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the standalone fspec binary
    // (binary built by cargo before integration tests run)

    // @step When I run `fspec discover-foundation --help`
    let output = Command::new(fspec_bin())
        .arg("discover-foundation")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn discover-foundation --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then stdout is byte-for-byte identical to tests/fixtures/help/discover-foundation.txt
    assert_eq!(code, 0, "discover-foundation --help must exit 0; stderr={stderr}");
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Two-front-doors scenario — discover-foundation-cli-subcommand.feature
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cli_bridge_delegates_to_same_fspec_core_function_as_dispatcher() {
    use codelet_fspec_core::{dispatch_command, DispatchRequest};

    // @step Given a project root tempdir with no spec/foundation.json.draft
    let tmp_dispatch = tempfile::tempdir().expect("tempdir");
    let tmp_binary = tempfile::tempdir().expect("tempdir");

    // @step When I dispatch discover-foundation via the dispatcher and via the standalone binary with identical flags
    let result = dispatch_command(DispatchRequest {
        command: "discover-foundation".to_string(),
        args_json: "{}".to_string(),
        project_root: tmp_dispatch.path().to_path_buf(),
    });
    assert!(result.success, "dispatcher path failed: {result:?}");
    let (code, stdout, stderr) = run_cmd(tmp_binary.path(), &[]);
    assert_eq!(code, 0, "binary path must exit 0; stdout={stdout} stderr={stderr}");

    // @step Then both invocations produce the same draft content on disk
    let dispatch_draft = read_to_string(&tmp_dispatch.path().join("spec/foundation.json.draft"));
    let binary_draft = read_to_string(&tmp_binary.path().join("spec/foundation.json.draft"));
    assert_eq!(dispatch_draft, binary_draft, "both front doors must write identical drafts");
}

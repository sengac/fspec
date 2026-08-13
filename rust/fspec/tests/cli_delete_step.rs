//! CLI surface for the `delete-step` subcommand on the standalone fspec
//! Rust binary — RPC-221.
//!
//! Feature: spec/features/delete-step-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim.
//!
//! PHASE B (TESTING): the CLI subcommand / core impl are still stubs, so the
//! behavioural scenarios are RED until PHASE C.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_delete_step(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("delete-step");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec delete-step");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_feature(project_root: &Path, rel: &str, body: &str) {
    let abs = project_root.join(rel);
    fs::create_dir_all(abs.parent().unwrap()).expect("mkdir feature parent");
    fs::write(&abs, body).expect("write feature file");
}

fn read_feature(project_root: &Path, rel: &str) -> String {
    fs::read_to_string(project_root.join(rel)).expect("read feature")
}

const LOGIN_GWT: &str = "Feature: Login\n\n  Scenario: Login\n    Given I am on the login page\n    When I enter valid credentials\n    Then I see the dashboard\n";

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/delete-step.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI deletes a step and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_deletes_a_step_and_prints_the_success_line() {
    // @step Given a tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", LOGIN_GWT);

    // @step When I run 'fspec delete-step spec/features/login.feature Login "When I enter valid credentials"' in that tempdir
    let (code, stdout, stderr) = run_delete_step(
        ws.path(),
        &[
            "spec/features/login.feature",
            "Login",
            "When I enter valid credentials",
        ],
    );

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Successfully deleted step'
    assert!(
        stdout.contains("✓ Successfully deleted step"),
        "stdout must contain success line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI surfaces a missing step with stderr Error prefix and exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_surfaces_a_missing_step_with_stderr_error_prefix_and_exit_1() {
    // @step Given a tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", LOGIN_GWT);

    // @step When I run 'fspec delete-step spec/features/login.feature Login "When nonexistent"' in that tempdir
    let (code, _stdout, stderr) = run_delete_step(
        ws.path(),
        &["spec/features/login.feature", "Login", "When nonexistent"],
    );

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error prefix; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_help_output_matches_captured_typescript_fixture_byte_for_byte() {
    // @step Given the standalone fspec Rust binary is built

    // @step When I run 'fspec delete-step --help'
    let output = Command::new(fspec_bin())
        .arg("delete-step")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn delete-step --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "delete-step --help must exit 0; stderr={stderr}");

    // @step And stdout matches the captured fixture at rust/fspec/tests/fixtures/help/delete-step.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_the_same_fspec_core_function_used_by_the_dispatcher() {
    // @step Given a project root tempdir with spec/features/login.feature whose scenario 'Login' has steps Given/When/Then
    let ws_cli = tempfile::tempdir().expect("tempdir cli");
    let ws_disp = tempfile::tempdir().expect("tempdir disp");
    write_feature(ws_cli.path(), "spec/features/login.feature", LOGIN_GWT);
    write_feature(ws_disp.path(), "spec/features/login.feature", LOGIN_GWT);

    // @step When I delete the same step once via the dispatcher and once via the CLI on identical inputs
    let req = codelet_fspec_core::DispatchRequest {
        command: "delete-step".to_string(),
        args_json: r#"{"feature":"spec/features/login.feature","scenario":"Login","step":"When I enter valid credentials"}"#.to_string(),
        project_root: ws_disp.path().to_path_buf(),
    };
    let _ = codelet_fspec_core::dispatch_command(req);
    let _ = run_delete_step(
        ws_cli.path(),
        &[
            "spec/features/login.feature",
            "Login",
            "When I enter valid credentials",
        ],
    );

    // @step Then both front doors produce the same resulting feature-file content
    let cli_after = read_feature(ws_cli.path(), "spec/features/login.feature");
    let disp_after = read_feature(ws_disp.path(), "spec/features/login.feature");
    // Honest red-phase guard: both front doors must have ACTUALLY removed the
    // step. Without this, two stubbed no-ops would leave both files unchanged
    // and compare equal, yielding a false green.
    assert!(
        !cli_after.contains("When I enter valid credentials"),
        "CLI front door must have deleted the step; got:\n{cli_after}"
    );
    assert!(
        !disp_after.contains("When I enter valid credentials"),
        "dispatcher front door must have deleted the step; got:\n{disp_after}"
    );
    assert_eq!(
        cli_after, disp_after,
        "CLI and dispatcher must produce identical feature-file content"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI surface does NOT expose --force (TS Commander defines no such
// option, so it is rejected as an unknown option with exit 1)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_force_flag_as_unknown_option() {
    // @step Given a tempdir with spec/features/login.feature containing a scenario with steps
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", LOGIN_GWT);
    let before = read_feature(ws.path(), "spec/features/login.feature");

    // @step When I run 'fspec delete-step spec/features/login.feature "Login" 1 --force' in that tempdir
    let (code, _stdout, stderr) = run_delete_step(
        ws.path(),
        &["spec/features/login.feature", "Login", "1", "--force"],
    );

    // @step Then the process exits with code 1
    assert_eq!(
        code, 1,
        "Commander rejects --force with exit 1; stderr={stderr}"
    );

    // @step And stderr contains the substring "unknown option '--force'"
    assert!(
        stderr.contains("unknown option '--force'"),
        "must reject --force as an unknown option; got:\n{stderr}"
    );

    // @step And the feature file is byte-identical to its pre-call content
    let after = read_feature(ws.path(), "spec/features/login.feature");
    assert_eq!(before, after, "feature file must be untouched on rejection");
}

//! CLI surface for the `generate-coverage` subcommand on the standalone
//! fspec Rust binary — RPC-231.
//!
//! Feature: spec/features/generate-coverage-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim.
//!
//! PHASE B (TESTING): the clap subcommand + CLI bridge are not yet wired,
//! so the behavioural scenarios are RED until PHASE C.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_generate(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("generate-coverage");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec generate-coverage");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_file(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

const FEATURE_ONE_SCENARIO: &str = "Feature: User Login

  Scenario: Login
    Given I am on the login page
    When I enter valid credentials
    Then I see the dashboard
";

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/generate-coverage.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_help_output_matches_the_captured_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec generate-coverage --help`
    let output = Command::new(fspec_bin())
        .arg("generate-coverage")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec generate-coverage --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "generate-coverage --help must exit 0; stderr={stderr}"
    );

    // @step And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/generate-coverage.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI creates a missing sidecar and prints the success report
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_creates_a_missing_sidecar_and_prints_the_success_report() {
    // @step Given a project root tempdir with a feature file "user-login.feature" and no coverage sidecar
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(
        ws.path(),
        "spec/features/user-login.feature",
        FEATURE_ONE_SCENARIO,
    );

    // @step When I run `fspec generate-coverage` in that tempdir
    let (code, stdout, stderr) = run_generate(ws.path(), &[]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}, stdout={stdout}");

    // @step And stdout contains the substring "Created 1"
    assert!(stdout.contains("Created 1"), "stdout={stdout}");

    // @step And the file spec/features/user-login.feature.coverage is created in that tempdir
    assert!(
        ws.path()
            .join("spec/features/user-login.feature.coverage")
            .exists(),
        "coverage sidecar must be created"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI forwards the --dry-run flag without writing files
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_forwards_the_dry_run_flag_without_writing_files() {
    // @step Given a project root tempdir with a feature file "user-login.feature" and no coverage sidecar
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(
        ws.path(),
        "spec/features/user-login.feature",
        FEATURE_ONE_SCENARIO,
    );

    // @step When I run `fspec generate-coverage --dry-run` in that tempdir
    let (code, stdout, stderr) = run_generate(ws.path(), &["--dry-run"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}, stdout={stdout}");

    // @step And stdout contains the substring "Would create 1 coverage files (DRY RUN)"
    assert!(
        stdout.contains("Would create 1 coverage files (DRY RUN)"),
        "stdout={stdout}"
    );

    // @step And no coverage sidecar file is created in that tempdir
    assert!(
        !ws.path()
            .join("spec/features/user-login.feature.coverage")
            .exists(),
        "dry-run must not write a sidecar"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports a missing features directory with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_a_missing_features_directory_with_exit_1() {
    // @step Given an empty project root tempdir with no spec/features directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec generate-coverage` in that tempdir
    let (code, stdout, stderr) = run_generate(ws.path(), &[]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "must exit 1; stderr={stderr}, stdout={stdout}");

    // @step And stderr contains the substring "Error:"
    assert!(stderr.contains("Error:"), "stderr={stderr}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_is_preserved_when_no_subcommand_is_provided() {
    // @step Given the fspec Rust binary has generate-coverage registered as a clap subcommand alongside other ported subcommands

    // @step When I run `fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec --help");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists generate-coverage as an available subcommand
    assert!(
        stdout.contains("generate-coverage"),
        "fspec --help must list generate-coverage; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_the_same_fspec_core_function_used_by_the_dispatcher() {
    // @step Given a project root tempdir with a feature file "user-login.feature" and no coverage sidecar
    let ws_disp = tempfile::tempdir().expect("tempdir disp");
    let ws_cli = tempfile::tempdir().expect("tempdir cli");
    for ws in [ws_disp.path(), ws_cli.path()] {
        write_file(ws, "spec/features/user-login.feature", FEATURE_ONE_SCENARIO);
    }

    // @step When I dispatch generate-coverage through fspec_core::dispatch::dispatch_command against that workspace
    let req = codelet_fspec_core::DispatchRequest {
        command: "generate-coverage".to_string(),
        args_json: "{}".to_string(),
        project_root: ws_disp.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And I run `fspec generate-coverage` against an identical workspace
    let (code, stdout, stderr) = run_generate(ws_cli.path(), &[]);
    assert_eq!(code, 0, "CLI must exit 0; stderr={stderr}, stdout={stdout}");

    // @step Then both invocations create the coverage sidecar
    assert!(
        ws_disp
            .path()
            .join("spec/features/user-login.feature.coverage")
            .exists(),
        "dispatcher path must create sidecar"
    );
    assert!(
        ws_cli
            .path()
            .join("spec/features/user-login.feature.coverage")
            .exists(),
        "CLI path must create sidecar"
    );

    // @step And the CLI bridge module rust/fspec/src/generate_coverage.rs contains NO inline scanning or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generate_coverage.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/generate_coverage.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Would create",
        "Created ",
        "system-reminder",
        "feature.coverage",
        "read_dir",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic)"
        );
    }
}

//! CLI surface for the `link-coverage` subcommand on the standalone
//! fspec Rust binary — RPC-240.
//!
//! Feature: spec/features/link-coverage-cli-subcommand.feature
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

fn run_link(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("link-coverage");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec link-coverage");
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

/// Story feature with one scenario "Login".
const FEATURE_STORY: &str = "@AUTH-001
Feature: User Login

  Scenario: Login
    Given I am on the login page
    When I enter valid credentials
    Then I see the dashboard
";

/// Test file whose @step comments match FEATURE_STORY's Login steps.
const TEST_MATCHING: &str = "// @step Given I am on the login page
// @step When I enter valid credentials
// @step Then I see the dashboard
test('login', () => {});
";

/// Empty sidecar listing scenario "Login" with no mappings.
const SIDECAR_LOGIN_EMPTY: &str = r#"{
  "scenarios": [
    { "name": "Login", "testMappings": [] }
  ],
  "stats": {
    "totalScenarios": 1,
    "coveredScenarios": 0,
    "coveragePercent": 0,
    "testFiles": [],
    "implFiles": [],
    "totalLinesCovered": 0
  }
}"#;

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/link-coverage.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_help_output_matches_the_captured_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec link-coverage --help`
    let output = Command::new(fspec_bin())
        .arg("link-coverage")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec link-coverage --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "link-coverage --help must exit 0; stderr={stderr}");

    // @step And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/link-coverage.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step And stdout contains the substring "--scenario"
    assert!(stdout.contains("--scenario"), "stdout={stdout}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI links a test mapping and prints the success message
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_links_a_test_mapping_and_prints_the_success_message() {
    // @step Given a project root tempdir has a feature file and coverage sidecar for scenario "Login" and a test file with matching @step comments
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/user-login.feature", FEATURE_STORY);
    write_file(
        ws.path(),
        "spec/features/user-login.feature.coverage",
        SIDECAR_LOGIN_EMPTY,
    );
    write_file(ws.path(), "src/auth.test.ts", TEST_MATCHING);

    // @step When I run `fspec link-coverage user-login --scenario Login --test-file src/auth.test.ts --test-lines 45-62` in that tempdir
    let (code, stdout, stderr) = run_link(
        ws.path(),
        &[
            "user-login",
            "--scenario",
            "Login",
            "--test-file",
            "src/auth.test.ts",
            "--test-lines",
            "45-62",
        ],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}, stdout={stdout}");

    // @step And stdout contains the substring "Linked test mapping"
    assert!(stdout.contains("Linked test mapping"), "stdout={stdout}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI without a valid flag combination exits 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_without_a_valid_flag_combination_exits_1() {
    // @step Given a project root tempdir has a coverage sidecar with scenario "Login"
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(
        ws.path(),
        "spec/features/user-login.feature.coverage",
        SIDECAR_LOGIN_EMPTY,
    );

    // @step When I run `fspec link-coverage user-login --scenario Login --impl-file src/login.ts` in that tempdir
    let (code, stdout, stderr) = run_link(
        ws.path(),
        &[
            "user-login",
            "--scenario",
            "Login",
            "--impl-file",
            "src/login.ts",
        ],
    );

    // @step Then the exit code is 1
    assert_eq!(code, 1, "must exit 1; stderr={stderr}, stdout={stdout}");

    // @step And stderr contains the substring "Error:"
    assert!(stderr.contains("Error:"), "stderr={stderr}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when the coverage sidecar is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_1_when_the_coverage_sidecar_is_missing() {
    // @step Given an empty project root tempdir with no coverage sidecar
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec link-coverage user-login --scenario Login --test-file src/auth.test.ts --test-lines 1-2 --skip-validation` in that tempdir
    let (code, stdout, stderr) = run_link(
        ws.path(),
        &[
            "user-login",
            "--scenario",
            "Login",
            "--test-file",
            "src/auth.test.ts",
            "--test-lines",
            "1-2",
            "--skip-validation",
        ],
    );

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
    // @step Given the fspec Rust binary has link-coverage registered as a clap subcommand alongside other ported subcommands

    // @step When I run `fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec --help");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists link-coverage as an available subcommand
    assert!(
        stdout.contains("link-coverage"),
        "fspec --help must list link-coverage; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_the_same_fspec_core_function_used_by_the_dispatcher() {
    // @step Given a project root tempdir has a feature file and coverage sidecar for scenario "Login" and a matching test file
    let ws_disp = tempfile::tempdir().expect("tempdir disp");
    let ws_cli = tempfile::tempdir().expect("tempdir cli");
    for ws in [ws_disp.path(), ws_cli.path()] {
        write_file(ws, "spec/features/user-login.feature", FEATURE_STORY);
        write_file(
            ws,
            "spec/features/user-login.feature.coverage",
            SIDECAR_LOGIN_EMPTY,
        );
        write_file(ws, "src/auth.test.ts", TEST_MATCHING);
    }

    // @step When I dispatch link-coverage through fspec_core::dispatch::dispatch_command against that workspace
    let req = codelet_fspec_core::DispatchRequest {
        command: "link-coverage".to_string(),
        args_json: r#"{"featureName":"user-login","scenario":"Login","testFile":"src/auth.test.ts","testLines":"45-62"}"#.to_string(),
        project_root: ws_disp.path().to_path_buf(),
    };
    let disp_result = codelet_fspec_core::dispatch_command(req);

    // @step And I run `fspec link-coverage user-login --scenario Login --test-file src/auth.test.ts --test-lines 45-62` against an identical workspace
    let (code, stdout, stderr) = run_link(
        ws_cli.path(),
        &[
            "user-login",
            "--scenario",
            "Login",
            "--test-file",
            "src/auth.test.ts",
            "--test-lines",
            "45-62",
        ],
    );

    // @step Then both invocations report success
    assert!(
        disp_result.success,
        "dispatcher path must succeed; got {disp_result:?}"
    );
    assert_eq!(code, 0, "CLI must exit 0; stderr={stderr}, stdout={stdout}");

    // @step And the CLI bridge module rust/fspec/src/link_coverage.rs contains NO inline mutation, validation, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/link_coverage.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/link_coverage.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Linked test mapping",
        "testMappings",
        "STEP VALIDATION",
        "Coverage file not found",
        "implMappings",
        "updateStats",
        "update_stats",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic)"
        );
    }
}

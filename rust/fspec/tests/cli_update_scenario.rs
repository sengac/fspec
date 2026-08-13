//! CLI surface for the `update-scenario` subcommand on the standalone fspec
//! Rust binary — RPC-314.
//!
//! Feature: spec/features/update-scenario-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file
//! above; @step comments mirror the Gherkin step text verbatim.
//!
//! RED PHASE: until the port lands and main.rs is wired, the binary has no
//! `update-scenario` subcommand, so these tests FAIL.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_update_scenario(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("update-scenario");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec update-scenario");
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

const USER_AUTH: &str = "Feature: Auth\n\n  Scenario: Login with valid credentials\n    Given I am on the login page\n    Then I should see the dashboard\n";

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/update-scenario.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI successfully renames a scenario and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_successfully_renames_a_scenario_and_prints_success_line() {
    // @step Given a tempdir with spec/features/user-auth.feature containing a scenario "Login with valid credentials"
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/user-auth.feature", USER_AUTH);

    // @step When I run 'fspec update-scenario spec/features/user-auth.feature "Login with valid credentials" "Login with email and password"' in that tempdir
    let (code, stdout, stderr) = run_update_scenario(
        ws.path(),
        &[
            "spec/features/user-auth.feature",
            "Login with valid credentials",
            "Login with email and password",
        ],
    );

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Successfully renamed scenario to 'Login with email and password' in user-auth.feature'
    assert!(
        stdout.contains("✓ Successfully renamed scenario to 'Login with email and password' in user-auth.feature"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And the file spec/features/user-auth.feature in the tempdir contains the line '  Scenario: Login with email and password'
    let after = read_feature(ws.path(), "spec/features/user-auth.feature");
    assert!(
        after
            .lines()
            .any(|l| l == "  Scenario: Login with email and password"),
        "expected renamed header line; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI surfaces a missing-file error with stderr Error prefix and exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_surfaces_a_missing_file_error_with_exit_1() {
    // @step Given a tempdir with no spec/features/missing.feature
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec/features/missing.feature").exists());

    // @step When I run 'fspec update-scenario spec/features/missing.feature "A" "B"' in that tempdir
    let (code, _stdout, stderr) =
        run_update_scenario(ws.path(), &["spec/features/missing.feature", "A", "B"]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Feature file not found:'
    assert!(
        stderr.contains("Feature file not found:"),
        "stderr must contain canonical not-found message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI surfaces a not-found scenario error with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_surfaces_a_not_found_scenario_error_with_exit_1() {
    // @step Given a tempdir with spec/features/user-auth.feature containing a scenario "Login with valid credentials"
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/user-auth.feature", USER_AUTH);

    // @step When I run 'fspec update-scenario spec/features/user-auth.feature "Nonexistent" "Whatever"' in that tempdir
    let (code, _stdout, stderr) = run_update_scenario(
        ws.path(),
        &["spec/features/user-auth.feature", "Nonexistent", "Whatever"],
    );

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Scenario 'Nonexistent' not found in feature file'
    assert!(
        stderr.contains("Scenario 'Nonexistent' not found in feature file"),
        "stderr must contain canonical not-found message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the standalone fspec Rust binary is built

    // @step When I run 'fspec update-scenario --help'
    let output = Command::new(fspec_bin())
        .arg("update-scenario")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn update-scenario --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(
        code, 0,
        "update-scenario --help must exit 0; stderr={stderr}"
    );

    // @step And stdout matches the captured fixture at rust/fspec/tests/fixtures/help/update-scenario.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/features/user-auth.feature containing a scenario "Login with valid credentials"
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/user-auth.feature", USER_AUTH);

    // @step When I dispatch update-scenario through fspec_core::dispatch::dispatch_command with feature='spec/features/user-auth.feature' oldName='Login with valid credentials' newName='Renamed'
    let req = codelet_fspec_core::DispatchRequest {
        command: "update-scenario".to_string(),
        args_json: r#"{"feature":"spec/features/user-auth.feature","oldName":"Login with valid credentials","newName":"Renamed"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher's DispatchResult.data parses to a structure whose message contains 'Successfully renamed scenario to 'Renamed' in user-auth.feature'
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Successfully renamed scenario to 'Renamed' in user-auth.feature"),
        "expected canonical message; got: {msg}"
    );

    // @step And the CLI bridge module rust/fspec/src/update_scenario.rs contains NO inline gherkin parsing or scenario-rename logic
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/update_scenario.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/update_scenario.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "parse_feature_lenient",
        "Feature::parse",
        "CoverageFile",
        "Scenario Outline",
        "scenarioToRename",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }

    // @step And the bridge module's only computation is JSON arg marshalling and CWD resolution
    // (Asserted indirectly by the forbidden-token sweep above.)
}

//! CLI surface for the `add-scenario` subcommand on the standalone fspec
//! Rust binary — RPC-190.
//!
//! Feature: spec/features/add-scenario-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file
//! above; @step comments mirror the Gherkin step text verbatim. At PHASE B
//! time the command is still a stub, so these tests are expected to FAIL
//! until PHASE C lands the implementation and the shared wiring.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::{fspec_bin, strip_comments};

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_add_scenario(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-scenario");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-scenario");
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

const FEATURE_LOGIN_PLAIN: &str = "Feature: Login\n  Scenario: A\n    Given x\n";
const TS_HELP_FIXTURE_AS: &str = include_str!("fixtures/help/add-scenario.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI adds a scenario and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_adds_a_scenario_and_prints_success_line() {
    // @step Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I run 'fspec add-scenario spec/features/login.feature "Login with invalid password"' in that tempdir
    let (code, stdout, stderr) = run_add_scenario(
        ws.path(),
        &["spec/features/login.feature", "Login with invalid password"],
    );

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Added scenario "Login with invalid password"'
    assert!(
        stdout.contains("✓ Added scenario \"Login with invalid password\""),
        "stdout must contain success line; got:\n{stdout}"
    );

    // @step And the file spec/features/login.feature in the tempdir contains the line '  Scenario: Login with invalid password'
    let after = fs::read_to_string(ws.path().join("spec/features/login.feature")).expect("read");
    assert!(
        after
            .lines()
            .any(|l| l == "  Scenario: Login with invalid password"),
        "missing scenario line:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints a warning when the scenario name already exists
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_a_warning_when_scenario_name_already_exists() {
    // @step Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I run 'fspec add-scenario spec/features/login.feature "A"' in that tempdir
    let (code, stdout, stderr) = run_add_scenario(ws.path(), &["spec/features/login.feature", "A"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'already exists in this feature'
    assert!(
        stdout.contains("already exists in this feature"),
        "stdout must contain warning; got:\n{stdout}"
    );

    // @step And stdout contains the substring '✓ Added scenario "A"'
    assert!(
        stdout.contains("✓ Added scenario \"A\""),
        "stdout must contain success line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI fails with exit 1 and Error prefix for a missing file
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_fails_with_exit_1_for_a_missing_file() {
    // @step Given a tempdir with NO spec/features/missing.feature file
    let ws = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(ws.path().join("spec/features")).expect("mkdir");

    // @step When I run 'fspec add-scenario spec/features/missing.feature "X"' in that tempdir
    let (code, _stdout, stderr) =
        run_add_scenario(ws.path(), &["spec/features/missing.feature", "X"]);

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
        "stderr must contain not-found message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the standalone fspec Rust binary is built

    // @step When I run 'fspec add-scenario --help'
    let output = Command::new(fspec_bin())
        .arg("add-scenario")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-scenario --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "add-scenario --help must exit 0; stderr={stderr}");

    // @step And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/add-scenario.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_AS);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        FEATURE_LOGIN_PLAIN,
    );

    // @step When I dispatch add-scenario through fspec_core::dispatch::dispatch_command with feature='spec/features/login.feature' and scenario='From dispatcher'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-scenario".to_string(),
        args_json: r#"{"feature":"spec/features/login.feature","scenario":"From dispatcher"}"#
            .to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher's DispatchResult.data parses to a structure whose success is true
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step And the CLI bridge module codelet/fspec/src/add_scenario.rs contains NO inline gherkin parsing or insertion logic
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_scenario.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_scenario.rs must exist as the CLI bridge module"
    );
    let bridge_src =
        strip_comments(&fs::read_to_string(&bridge_path).expect("bridge module readable"));
    for forbidden in [
        "parse_feature_lenient",
        "Feature::parse",
        "Scenario Outline",
        "[precondition]",
        "insertIndex",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }

    // @step And the bridge module's only computation is JSON arg marshalling and CWD resolution
    // (Asserted indirectly by the forbidden-token sweep above.)
}

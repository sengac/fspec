//! CLI surface for the `add-step` subcommand on the standalone fspec
//! Rust binary — RPC-192.
//!
//! Feature: spec/features/add-step-cli-subcommand.feature
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

fn run_add_step(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-step");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-step");
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

const FEATURE_PLAIN: &str = "Feature: Login\n  Scenario: Login\n    Given x\n";
const TS_HELP_FIXTURE_ASTEP: &str = include_str!("fixtures/help/add-step.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI adds a step and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_adds_a_step_and_prints_success_line() {
    // @step Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", FEATURE_PLAIN);

    // @step When I run 'fspec add-step spec/features/login.feature "Login" given "I am on the login page"' in that tempdir
    let (code, stdout, stderr) = run_add_step(
        ws.path(),
        &[
            "spec/features/login.feature",
            "Login",
            "given",
            "I am on the login page",
        ],
    );

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Added given step to scenario "Login"'
    assert!(
        stdout.contains("✓ Added given step to scenario \"Login\""),
        "stdout must contain success line; got:\n{stdout}"
    );

    // @step And the file spec/features/login.feature in the tempdir contains the line 'Given I am on the login page'
    let after = fs::read_to_string(ws.path().join("spec/features/login.feature")).expect("read");
    assert!(
        after.contains("Given I am on the login page"),
        "missing step text:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects an invalid step type with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_an_invalid_step_type_with_exit_1() {
    // @step Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", FEATURE_PLAIN);

    // @step When I run 'fspec add-step spec/features/login.feature "Login" maybe "whatever"' in that tempdir
    let (code, _stdout, stderr) = run_add_step(
        ws.path(),
        &["spec/features/login.feature", "Login", "maybe", "whatever"],
    );

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Invalid step type: "maybe"'
    assert!(
        stderr.contains("Invalid step type: \"maybe\""),
        "stderr must contain canonical invalid-type message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects an unknown scenario with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_an_unknown_scenario_with_exit_1() {
    // @step Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", FEATURE_PLAIN);

    // @step When I run 'fspec add-step spec/features/login.feature "Nope" given "x"' in that tempdir
    let (code, _stdout, stderr) = run_add_step(
        ws.path(),
        &["spec/features/login.feature", "Nope", "given", "x"],
    );

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Scenario not found: "Nope"'
    assert!(
        stderr.contains("Scenario not found: \"Nope\""),
        "stderr must contain canonical scenario-not-found message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the standalone fspec Rust binary is built

    // @step When I run 'fspec add-step --help'
    let output = Command::new(fspec_bin())
        .arg("add-step")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-step --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "add-step --help must exit 0; stderr={stderr}");

    // @step And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/add-step.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_ASTEP);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: Login\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", FEATURE_PLAIN);

    // @step When I dispatch add-step through fspec_core::dispatch::dispatch_command with feature='spec/features/login.feature', scenario='Login', type='when' and text='I act'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-step".to_string(),
        args_json: r#"{"feature":"spec/features/login.feature","scenario":"Login","type":"when","text":"I act"}"#
            .to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher's DispatchResult.data parses to a structure whose success is true
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true));

    // @step And the CLI bridge module codelet/fspec/src/add_step.rs contains NO inline gherkin parsing, placeholder, or insertion logic
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_step.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_step.rs must exist as the CLI bridge module"
    );
    let bridge_src =
        strip_comments(&fs::read_to_string(&bridge_path).expect("bridge module readable"));
    for forbidden in [
        "parse_feature_lenient",
        "Feature::parse",
        "[precondition]",
        "[expected outcome]",
        "VALID_STEP_TYPES",
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

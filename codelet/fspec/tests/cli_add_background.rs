//! CLI surface for the `add-background` subcommand on the standalone fspec
//! Rust binary — RPC-171.
//!
//! Feature: spec/features/add-background-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file
//! above; @step comments mirror the Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_add_background(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-background");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-background");
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

const FEATURE_LOGIN_PLAIN: &str = "Feature: Login\n  Scenario: A\n    Given x\n";

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/add-background.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI successfully adds a Background and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_successfully_adds_background_and_prints_success_line() {
    // @step Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", FEATURE_LOGIN_PLAIN);

    // @step When I run 'fspec add-background spec/features/login.feature "As a user\nI want to log in\nSo that I access my account"' in that tempdir
    let (code, stdout, stderr) = run_add_background(
        ws.path(),
        &[
            "spec/features/login.feature",
            "As a user\nI want to log in\nSo that I access my account",
        ],
    );

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Added background to spec/features/login.feature'
    assert!(
        stdout.contains("✓ Added background to spec/features/login.feature"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And the file spec/features/login.feature in the tempdir contains the line '  Background: User Story'
    let after = read_feature(ws.path(), "spec/features/login.feature");
    assert!(
        after.lines().any(|l| l == "  Background: User Story"),
        "feature file must gain Background line; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI resolves a bare feature name by basename
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_resolves_bare_feature_name_by_basename() {
    // @step Given a tempdir with spec/features/dashboard.feature containing 'Feature: Dashboard\n  Scenario: A\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/dashboard.feature",
        "Feature: Dashboard\n  Scenario: A\n    Given x\n",
    );

    // @step When I run 'fspec add-background dashboard "As a user\nI want a dashboard\nSo that I see overview"' in that tempdir
    let (code, stdout, stderr) = run_add_background(
        ws.path(),
        &["dashboard", "As a user\nI want a dashboard\nSo that I see overview"],
    );

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Added background to dashboard'
    assert!(
        stdout.contains("✓ Added background to dashboard"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And the file spec/features/dashboard.feature in the tempdir contains the line '  Background: User Story'
    let after = read_feature(ws.path(), "spec/features/dashboard.feature");
    assert!(
        after.lines().any(|l| l == "  Background: User Story"),
        "feature file must gain Background line; got:\n{after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI surfaces empty-text error with stderr Error prefix and exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_surfaces_empty_text_error() {
    // @step Given a tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", FEATURE_LOGIN_PLAIN);

    // @step When I run 'fspec add-background spec/features/login.feature ""' in that tempdir
    let (code, _stdout, stderr) = run_add_background(ws.path(), &["spec/features/login.feature", ""]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "stderr must contain Error prefix; got:\n{stderr}");

    // @step And stderr contains the substring 'Background text cannot be empty'
    assert!(
        stderr.contains("Background text cannot be empty"),
        "stderr must contain canonical empty-text message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI surfaces not-found error with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_surfaces_not_found_error() {
    // @step Given a tempdir with NO spec/features/missing.feature file
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run 'fspec add-background spec/features/missing.feature "As a user"' in that tempdir
    let (code, _stdout, stderr) =
        run_add_background(ws.path(), &["spec/features/missing.feature", "As a user"]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "stderr must contain Error prefix; got:\n{stderr}");

    // @step And stderr contains the substring 'Feature file not found: spec/features/missing.feature'
    assert!(
        stderr.contains("Feature file not found: spec/features/missing.feature"),
        "stderr must contain canonical not-found message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_help_matches_ts_fixture() {
    // @step Given the standalone fspec Rust binary is built

    // @step When I run 'fspec add-background --help'
    let output = Command::new(fspec_bin())
        .arg("add-background")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-background --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "add-background --help must exit 0; stderr={stderr}");

    // @step And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/add-background.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/features/login.feature containing 'Feature: Login\n  Scenario: A\n    Given x\n'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", FEATURE_LOGIN_PLAIN);

    // @step When I dispatch add-background through fspec_core::dispatch::dispatch_command with feature='spec/features/login.feature' and text='As a user'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-background".to_string(),
        args_json: r#"{"feature":"spec/features/login.feature","text":"As a user"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher's DispatchResult.data parses to a structure whose message contains 'Added background to spec/features/login.feature'
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("parse data json");
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Added background to spec/features/login.feature"),
        "expected canonical message; got: {msg}"
    );

    // @step And the CLI bridge module codelet/fspec/src/add_background.rs contains NO inline gherkin parsing or line-splice mutation logic
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_background.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_background.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "parse_feature_lenient",
        "Feature::parse",
        "Background: User Story",
        "glob_feature_files",
        "splice",
        "docStringEndIndex",
        ".split('\\n')",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }

    // @step And the bridge module's only computation is JSON arg marshalling and CWD resolution
    // (Asserted indirectly by the forbidden-token sweep above.)
}

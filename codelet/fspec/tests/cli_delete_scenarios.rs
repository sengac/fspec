//! CLI surface for the `delete-scenarios` subcommand on the standalone fspec
//! Rust binary — RPC-220.
//!
//! Feature: spec/features/delete-scenarios-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim.
//!
//! delete-scenarios has NO custom -help.ts; its --help is bare Commander.js,
//! hard-coded as DELETE_SCENARIOS_HELP in main.rs (mirrors delete-features).
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

fn run_delete_scenarios(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("delete-scenarios");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec delete-scenarios");
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
    fs::read_to_string(project_root.join(rel)).expect("read feature file")
}

/// Feature with two @spike scenarios and one untagged scenario.
fn two_spike_one_plain() -> String {
    "Feature: Demo\n\n  @spike\n  Scenario: First spike\n    Given a precondition\n\n  @spike\n  Scenario: Second spike\n    Given another precondition\n\n  Scenario: Plain keeper\n    Given an untagged precondition\n".to_string()
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/delete-scenarios.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI dry-run previews deletions without removing scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_dry_run_previews_deletions_without_removing_scenarios() {
    // @step Given a tempdir with one feature containing two @spike scenarios
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/demo.feature", &two_spike_one_plain());

    // @step When I run 'fspec delete-scenarios --tag @spike --dry-run' in that tempdir
    let (code, stdout, stderr) = run_delete_scenarios(ws.path(), &["--tag", "@spike", "--dry-run"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'Dry run mode - no files modified'
    assert!(
        stdout.contains("Dry run mode - no files modified"),
        "stdout must announce dry-run; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Would delete 2 scenario(s) from 1 file(s):'
    assert!(
        stdout.contains("Would delete 2 scenario(s) from 1 file(s):"),
        "stdout must show would-delete header; got:\n{stdout}"
    );

    // @step And the feature file still contains both @spike scenarios
    let after = read_feature(ws.path(), "spec/features/demo.feature");
    assert!(after.contains("First spike"), "got:\n{after}");
    assert!(after.contains("Second spike"), "got:\n{after}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI deletes matching scenarios and prints the success message
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_deletes_matching_scenarios_and_prints_the_success_message() {
    // @step Given a tempdir with one feature containing two @spike scenarios and one untagged scenario
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/demo.feature", &two_spike_one_plain());

    // @step When I run 'fspec delete-scenarios --tag @spike' in that tempdir
    let (code, stdout, stderr) = run_delete_scenarios(ws.path(), &["--tag", "@spike"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Deleted 2 scenario(s) from 1 file(s)'
    assert!(
        stdout.contains("✓ Deleted 2 scenario(s) from 1 file(s)"),
        "stdout must show success message; got:\n{stdout}"
    );

    // @step And the feature file no longer contains the @spike scenarios
    let after = read_feature(ws.path(), "spec/features/demo.feature");
    assert!(!after.contains("First spike"), "got:\n{after}");
    assert!(!after.contains("Second spike"), "got:\n{after}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI with no --tag exits 1 with stderr Error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_with_no_tag_exits_1_with_stderr_error_prefix() {
    // @step Given a tempdir with a feature tagged @spike
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/demo.feature", &two_spike_one_plain());

    // @step When I run 'fspec delete-scenarios' in that tempdir
    let (code, _stdout, stderr) = run_delete_scenarios(ws.path(), &[]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error prefix; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_the_same_fspec_core_function_used_by_the_dispatcher() {
    // @step Given a project root tempdir with one feature containing two @spike scenarios
    let ws_cli = tempfile::tempdir().expect("tempdir cli");
    let ws_disp = tempfile::tempdir().expect("tempdir disp");
    for ws in [ws_cli.path(), ws_disp.path()] {
        write_feature(ws, "spec/features/demo.feature", &two_spike_one_plain());
    }

    // @step When I run a dry-run delete-scenarios once via the dispatcher and once via the CLI on identical inputs
    let req = codelet_fspec_core::DispatchRequest {
        command: "delete-scenarios".to_string(),
        args_json: r#"{"tags":["@spike"],"dryRun":true}"#.to_string(),
        project_root: ws_disp.path().to_path_buf(),
    };
    let disp_result = codelet_fspec_core::dispatch_command(req);
    let (_code, cli_stdout, _stderr) =
        run_delete_scenarios(ws_cli.path(), &["--tag", "@spike", "--dry-run"]);

    // @step Then both front doors report the same deletedCount and fileCount
    let disp_data: serde_json::Value =
        serde_json::from_str(&disp_result.data).unwrap_or(serde_json::Value::Null);
    let disp_count = disp_data["deletedCount"].as_u64().unwrap_or(u64::MAX);
    let disp_files = disp_data["fileCount"].as_u64().unwrap_or(u64::MAX);
    assert!(
        cli_stdout.contains(&format!(
            "Would delete {disp_count} scenario(s) from {disp_files} file(s):"
        )),
        "CLI dry-run header must match dispatcher deletedCount={disp_count} fileCount={disp_files}; got CLI stdout:\n{cli_stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI help output matches the captured bare-Commander fixture byte-for-byte
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_help_output_matches_the_captured_bare_commander_fixture_byte_for_byte() {
    // @step Given the standalone fspec Rust binary is built

    // @step When I run 'fspec delete-scenarios --help'
    let output = Command::new(fspec_bin())
        .arg("delete-scenarios")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn delete-scenarios --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "delete-scenarios --help must exit 0; stderr={stderr}");

    // @step And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/delete-scenarios.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

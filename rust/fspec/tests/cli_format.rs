//! CLI surface for the `format` subcommand on the standalone fspec Rust
//! binary — RPC-230.
//!
//! Feature: spec/features/format-cli-subcommand.feature
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

fn run_format(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("format");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec format");
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

/// A well-formed but non-canonically-indented feature (steps under-indented
/// to 2 spaces; the formatter must renormalise them to 4 spaces).
fn messy_feature(name: &str) -> String {
    format!("Feature: {name}\n\n  Scenario: A\n  Given x\n  When y\n  Then z\n")
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/format.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI formats all feature files and prints the green summary
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_formats_all_feature_files_and_prints_the_green_summary() {
    // @step Given a temp workspace contains two well-formed feature files under spec/features
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/one.feature",
        &messy_feature("One"),
    );
    write_feature(
        ws.path(),
        "spec/features/two.feature",
        &messy_feature("Two"),
    );

    // @step When I run `./rust/target/release/fspec format` from that workspace
    let (code, stdout, stderr) = run_format(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Formatted 2 feature files'
    assert!(
        stdout.contains("✓ Formatted 2 feature files"),
        "stdout must show summary; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI formats a single supplied file
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_formats_a_single_supplied_file() {
    // @step Given a temp workspace contains spec/features/login.feature
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        &messy_feature("Login"),
    );

    // @step When I run `./rust/target/release/fspec format spec/features/login.feature` from that workspace
    let (code, stdout, stderr) = run_format(ws.path(), &["spec/features/login.feature"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Formatted spec/features/login.feature'
    assert!(
        stdout.contains("✓ Formatted spec/features/login.feature"),
        "stdout must show single-file message; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints a no-files message when none are found
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_a_no_files_message_when_none_are_found() {
    // @step Given an empty directory with no spec/features feature files is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./rust/target/release/fspec format` from that directory
    let (code, stdout, stderr) = run_format(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'No feature files found to format'
    assert!(
        stdout.contains("No feature files found to format"),
        "stdout must show no-files message; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI errors when a supplied file is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_errors_when_a_supplied_file_is_missing() {
    // @step Given a temp workspace with no spec/features/missing.feature file
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./rust/target/release/fspec format spec/features/missing.feature` from that workspace
    let (code, _stdout, stderr) = run_format(ws.path(), &["spec/features/missing.feature"]);

    // @step Then the command exits with a non-zero status
    assert_ne!(code, 0, "expected non-zero exit; stderr={stderr}");

    // @step And stderr contains the substring 'Error: File not found: spec/features/missing.feature'
    assert!(
        stderr.contains("Error: File not found: spec/features/missing.feature"),
        "stderr must show file-not-found error; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_the_same_fspec_core_function_used_by_the_dispatcher() {
    // @step Given a project root tempdir with two well-formed feature files under spec/features
    let ws_cli = tempfile::tempdir().expect("tempdir cli");
    let ws_disp = tempfile::tempdir().expect("tempdir disp");
    for ws in [ws_cli.path(), ws_disp.path()] {
        write_feature(ws, "spec/features/one.feature", &messy_feature("One"));
        write_feature(ws, "spec/features/two.feature", &messy_feature("Two"));
    }

    // @step When I run format once via the dispatcher and once via the CLI on identical inputs
    let req = codelet_fspec_core::DispatchRequest {
        command: "format".to_string(),
        args_json: "{}".to_string(),
        project_root: ws_disp.path().to_path_buf(),
    };
    let _ = codelet_fspec_core::dispatch_command(req);
    let (_code, _stdout, _stderr) = run_format(ws_cli.path(), &[]);

    // @step Then both front doors rewrite the files to identical content
    for rel in ["spec/features/one.feature", "spec/features/two.feature"] {
        let disp = read_feature(ws_disp.path(), rel);
        let cli = read_feature(ws_cli.path(), rel);
        assert_eq!(
            disp, cli,
            "front doors must produce identical {rel} content"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: format --help is byte-for-byte identical to the TS reference
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_format_help_is_byte_for_byte_identical() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec format --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("format")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn format --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "format --help must exit 0; stderr={stderr}");

    // @step And stdout matches the captured fixture at rust/fspec/tests/fixtures/help/format.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

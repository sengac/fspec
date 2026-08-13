//! CLI surface for the `delete-features` subcommand on the standalone fspec
//! Rust binary — RPC-218.
//!
//! Feature: spec/features/delete-features-cli-subcommand.feature
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

fn run_delete_features(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("delete-features");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec delete-features");
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

fn exists(project_root: &Path, rel: &str) -> bool {
    project_root.join(rel).exists()
}

fn tagged(tags: &str, name: &str) -> String {
    format!("{tags}\nFeature: {name}\n\n  Scenario: A\n    Given x\n")
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/delete-features.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI dry-run previews deletions without removing files
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_dry_run_previews_deletions_without_removing_files() {
    // @step Given a tempdir with two features tagged @deprecated
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/a.feature",
        &tagged("@deprecated", "A"),
    );
    write_feature(
        ws.path(),
        "spec/features/b.feature",
        &tagged("@deprecated", "B"),
    );

    // @step When I run 'fspec delete-features --tag @deprecated --dry-run' in that tempdir
    let (code, stdout, stderr) =
        run_delete_features(ws.path(), &["--tag", "@deprecated", "--dry-run"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'Dry run mode - no files modified'
    assert!(
        stdout.contains("Dry run mode - no files modified"),
        "stdout must announce dry-run; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Would delete 2 feature file(s):'
    assert!(
        stdout.contains("Would delete 2 feature file(s):"),
        "stdout must show would-delete header; got:\n{stdout}"
    );

    // @step And both feature files still exist on disk
    assert!(exists(ws.path(), "spec/features/a.feature"));
    assert!(exists(ws.path(), "spec/features/b.feature"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI deletes matching features and lists them
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_deletes_matching_features_and_lists_them() {
    // @step Given a tempdir with two features tagged @deprecated
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/a.feature",
        &tagged("@deprecated", "A"),
    );
    write_feature(
        ws.path(),
        "spec/features/b.feature",
        &tagged("@deprecated", "B"),
    );

    // @step When I run 'fspec delete-features --tag @deprecated' in that tempdir
    let (code, stdout, stderr) = run_delete_features(ws.path(), &["--tag", "@deprecated"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Deleted 2 feature file(s)'
    assert!(
        stdout.contains("✓ Deleted 2 feature file(s)"),
        "stdout must show deleted count; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Deleted files:'
    assert!(
        stdout.contains("Deleted files:"),
        "stdout must show deleted-files header; got:\n{stdout}"
    );

    // @step And both feature files no longer exist on disk
    assert!(!exists(ws.path(), "spec/features/a.feature"));
    assert!(!exists(ws.path(), "spec/features/b.feature"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI with no --tag exits 1 with stderr Error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_with_no_tag_exits_1_with_stderr_error_prefix() {
    // @step Given a tempdir with a feature tagged @deprecated
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/a.feature",
        &tagged("@deprecated", "A"),
    );

    // @step When I run 'fspec delete-features' in that tempdir
    let (code, _stdout, stderr) = run_delete_features(ws.path(), &[]);

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

    // @step When I run 'fspec delete-features --help'
    let output = Command::new(fspec_bin())
        .arg("delete-features")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn delete-features --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(
        code, 0,
        "delete-features --help must exit 0; stderr={stderr}"
    );

    // @step And stdout matches the captured fixture at rust/fspec/tests/fixtures/help/delete-features.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_the_same_fspec_core_function_used_by_the_dispatcher() {
    // @step Given a project root tempdir with two features tagged @deprecated
    let ws_cli = tempfile::tempdir().expect("tempdir cli");
    let ws_disp = tempfile::tempdir().expect("tempdir disp");
    for ws in [ws_cli.path(), ws_disp.path()] {
        write_feature(ws, "spec/features/a.feature", &tagged("@deprecated", "A"));
        write_feature(ws, "spec/features/b.feature", &tagged("@deprecated", "B"));
    }

    // @step When I run a dry-run delete-features once via the dispatcher and once via the CLI on identical inputs
    let req = codelet_fspec_core::DispatchRequest {
        command: "delete-features".to_string(),
        args_json: r#"{"tags":["@deprecated"],"dryRun":true}"#.to_string(),
        project_root: ws_disp.path().to_path_buf(),
    };
    let disp_result = codelet_fspec_core::dispatch_command(req);
    let (_code, cli_stdout, _stderr) =
        run_delete_features(ws_cli.path(), &["--tag", "@deprecated", "--dry-run"]);

    // @step Then both front doors report the same matching files and deletedCount
    let disp_data: serde_json::Value =
        serde_json::from_str(&disp_result.data).unwrap_or(serde_json::Value::Null);
    let disp_count = disp_data["deletedCount"].as_u64().unwrap_or(u64::MAX);
    assert!(
        cli_stdout.contains(&format!("Would delete {disp_count} feature file(s):")),
        "CLI dry-run count must match dispatcher deletedCount={disp_count}; got CLI stdout:\n{cli_stdout}"
    );
    let disp_files = disp_data["files"].as_array().cloned().unwrap_or_default();
    for f in &disp_files {
        if let Some(rel) = f.as_str() {
            assert!(
                cli_stdout.contains(rel),
                "CLI dry-run output must list dispatcher file {rel}; got:\n{cli_stdout}"
            );
        }
    }
}

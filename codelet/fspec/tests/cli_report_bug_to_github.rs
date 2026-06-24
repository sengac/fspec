//! CLI surface for the `report-bug-to-github` subcommand on the standalone
//! fspec Rust binary — RPC-285 (DETERMINISTIC-CORE scope).
//!
//! Feature: spec/features/report-bug-to-github-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario; @step comments mirror the
//! Gherkin step text verbatim.
//!
//! RED PHASE: the command is still a stub / unwired; these tests FAIL now.
//!
//! SCOPE: the CLI prints the gathering banner and the constructed GitHub URL;
//! it never launches a browser (deferred, same class as research EXECUTE).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_cmd(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("report-bug-to-github");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    cmd.env_remove("CLICOLOR_FORCE");
    cmd.env("NO_COLOR", "1");
    let output = cmd.output().expect("spawn fspec report-bug-to-github");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/report-bug-to-github.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenarios — report-bug-to-github-cli-subcommand.feature
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cli_prints_gathering_banner_and_exits_successfully() {
    // @step Given an empty project root tempdir
    let tmp = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec report-bug-to-github --bug-description "crash on save"` in that directory
    let (code, stdout, stderr) = run_cmd(tmp.path(), &["--bug-description", "crash on save"]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stdout={stdout} stderr={stderr}");

    // @step And stdout contains "Gathering system context..."
    assert!(
        stdout.contains("Gathering system context..."),
        "stdout={stdout}"
    );
}

#[test]
fn cli_output_includes_constructed_github_issue_url() {
    // @step Given an empty project root tempdir
    let tmp = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec report-bug-to-github --bug-description "crash on save"` in that directory
    let (code, stdout, stderr) = run_cmd(tmp.path(), &["--bug-description", "crash on save"]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stdout={stdout} stderr={stderr}");

    // @step And stdout contains "https://github.com/sengac/fspec/issues/new?title="
    assert!(
        stdout.contains("https://github.com/sengac/fspec/issues/new?title="),
        "stdout={stdout}"
    );
}

#[test]
fn cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the standalone fspec binary
    // (binary built by cargo before integration tests run)

    // @step When I run `fspec report-bug-to-github --help`
    let output = Command::new(fspec_bin())
        .arg("report-bug-to-github")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn report-bug-to-github --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits with code 0
    assert_eq!(
        code, 0,
        "report-bug-to-github --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to tests/fixtures/help/report-bug-to-github.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

//! CLI surface for the `research` subcommand on the standalone fspec Rust
//! binary — RPC-286 (LIST-only scope).
//!
//! Feature: spec/features/research-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario; @step comments mirror the
//! Gherkin step text verbatim.
//!
//! RED PHASE: the command is still a stub / unwired; these tests FAIL now.

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
    cmd.arg("research");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    cmd.env_remove("CLICOLOR_FORCE");
    cmd.env("NO_COLOR", "1");
    let output = cmd.output().expect("spawn fspec research");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/research.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenarios — research-cli-subcommand.feature
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cli_lists_available_research_tools_with_header() {
    // @step Given an empty project root tempdir
    let tmp = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec research` in that directory
    let (code, stdout, stderr) = run_cmd(tmp.path(), &[]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stdout={stdout} stderr={stderr}");

    // @step And stdout contains "Available Research Tools:"
    assert!(stdout.contains("Available Research Tools:"), "stdout={stdout}");

    // @step And stdout contains "ast"
    assert!(stdout.contains("ast"), "stdout={stdout}");

    // @step And stdout contains "perplexity"
    assert!(stdout.contains("perplexity"), "stdout={stdout}");

    // @step And stdout contains "stakeholder"
    assert!(stdout.contains("stakeholder"), "stdout={stdout}");
}

#[test]
fn cli_tool_listing_includes_per_tool_usage_guidance() {
    // @step Given an empty project root tempdir
    let tmp = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec research` in that directory
    let (code, stdout, stderr) = run_cmd(tmp.path(), &[]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stdout={stdout} stderr={stderr}");

    // @step And stdout contains "Usage: fspec research --tool=ast <args>"
    assert!(
        stdout.contains("Usage: fspec research --tool=ast <args>"),
        "stdout={stdout}"
    );
}

#[test]
fn cli_fails_with_not_found_error_for_unknown_tool() {
    // @step Given an empty project root tempdir
    let tmp = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec research --tool does-not-exist` in that directory
    let (code, stdout, stderr) = run_cmd(tmp.path(), &["--tool", "does-not-exist"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout} stderr={stderr}");

    // @step And stderr contains "Research tool not found: does-not-exist"
    assert!(
        stderr.contains("Research tool not found: does-not-exist"),
        "stderr={stderr}"
    );
}

#[test]
fn cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the standalone fspec binary
    // (binary built by cargo before integration tests run)

    // @step When I run `fspec research --help`
    let output = Command::new(fspec_bin())
        .arg("research")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn research --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "research --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to tests/fixtures/help/research.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

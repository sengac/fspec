//! CLI surface for the `register-tag` subcommand on the standalone fspec
//! Rust binary — RPC-265.
//!
//! Feature: spec/features/register-tag-cli-subcommand.feature
//!
//! These tests exercise the wired-up clap subcommand and the ported
//! `rust/fspec-core/src/commands/register_tag.rs` implementation
//! through the standalone fspec binary. Each scenario maps 1:1 to a
//! Gherkin scenario in the feature file above; @step comments mirror
//! the Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_register_tag(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("register-tag");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec register-tag");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI successfully registers a new tag and prints the multi-line success block
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cli_successfully_registers_new_tag_and_prints_success_block() {
    // @step Given a tempdir with no spec/tags.json
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec/tags.json").exists());

    // @step When I run 'fspec register-tag @ws "Technical Tags" "WebSocket features"' in that tempdir
    let (code, stdout, stderr) =
        run_register_tag(ws.path(), &["@ws", "Technical Tags", "WebSocket features"]);

    // @step Then the process exits with code 0
    assert_eq!(
        code, 0,
        "fspec register-tag must exit 0 on the happy path; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring '✓ Successfully registered @ws in Technical Tags'
    assert!(
        stdout.contains("✓ Successfully registered @ws in Technical Tags"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Updated: spec/tags.json'
    assert!(
        stdout.contains("Updated: spec/tags.json"),
        "stdout must contain Updated line; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Regenerated: spec/TAGS.md'
    assert!(
        stdout.contains("Regenerated: spec/TAGS.md"),
        "stdout must contain Regenerated line; got:\n{stdout}"
    );

    // @step And spec/tags.json exists in the tempdir
    assert!(ws.path().join("spec/tags.json").exists());

    // @step And spec/TAGS.md exists in the tempdir
    assert!(ws.path().join("spec/TAGS.md").exists());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects invalid tag format with stderr Error prefix and exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cli_rejects_invalid_tag_format_with_error_prefix_and_exit_1() {
    // @step Given a tempdir with no spec/tags.json
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run 'fspec register-tag InvalidTag "Technical Tags" "desc"' in that tempdir
    let (code, stdout, stderr) =
        run_register_tag(ws.path(), &["InvalidTag", "Technical Tags", "desc"]);

    // @step Then the process exits with code 1
    assert_eq!(
        code, 1,
        "fspec register-tag must exit 1 on invalid tag; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Invalid tag format'
    assert!(
        stderr.contains("Invalid tag format"),
        "stderr must contain 'Invalid tag format'; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports lowercase conversion note when tag contained uppercase characters
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cli_reports_lowercase_conversion_note() {
    // @step Given a tempdir with no spec/tags.json
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run 'fspec register-tag @API-Integration "Technical Tags" "API"' in that tempdir
    let (code, stdout, stderr) =
        run_register_tag(ws.path(), &["@API-Integration", "Technical Tags", "API"]);

    // @step Then the process exits with code 0
    assert_eq!(
        code, 0,
        "fspec register-tag must exit 0 on uppercase conversion; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring '✓ Successfully registered @api-integration (converted from @API-Integration) in Technical Tags'
    assert!(
        stdout.contains(
            "✓ Successfully registered @api-integration (converted from @API-Integration) in Technical Tags"
        ),
        "stdout must contain canonical converted-success line; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Note: Tag converted to lowercase: @API-Integration → @api-integration'
    assert!(
        stdout.contains("Note: Tag converted to lowercase: @API-Integration → @api-integration"),
        "stdout must contain lowercase-conversion note; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_RT: &str = include_str!("fixtures/help/register-tag.txt");

#[test]
fn cli_help_output_matches_captured_typescript_fixture() {
    // @step Given the standalone fspec Rust binary is built
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run 'fspec register-tag --help'
    let output = Command::new(fspec_bin())
        .arg("register-tag")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn register-tag --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "register-tag --help must exit 0; stderr={stderr}");

    // @step And stdout matches the captured fixture at rust/fspec/tests/fixtures/help/register-tag.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_RT);
}

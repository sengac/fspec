//! CLI surface for the `bootstrap` subcommand on the standalone fspec Rust
//! binary — RPC-200.
//!
//! Feature: spec/features/bootstrap-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario; @step comments mirror the
//! Gherkin step text verbatim.
//!
//! RED PHASE (Phase B): the `bootstrap` clap subcommand is not wired until
//! Phase C and the core impl is still the 1-arg NotYetPorted stub (and the
//! byte-exact bootstrap_doc.txt asset is not yet captured), so these tests are
//! EXPECTED to fail until then.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

mod common;

use common::fspec_bin;

const TS_HELP_FIXTURE_BOOTSTRAP: &str = include_str!("fixtures/help/bootstrap.txt");

// ---------- scenarios ----------

#[test]
fn scenario_clap_exposes_bootstrap_and_prints_byte_parity_help() {
    // @step Given the fspec Rust binary has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `fspec bootstrap --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("bootstrap")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec bootstrap --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "bootstrap --help must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/bootstrap.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_BOOTSTRAP);

    // @step Then stdout starts with a blank line followed by "BOOTSTRAP"
    assert!(
        stdout.starts_with("\nBOOTSTRAP\n"),
        "help must start with blank line then BOOTSTRAP; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_prints_the_complete_documentation_and_exits_0() {
    // @step Given a temp working directory with no fspec-config.json and no foundation.json
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec bootstrap` from that directory
    let output = Command::new(fspec_bin())
        .arg("bootstrap")
        .current_dir(ws.path())
        .output()
        .expect("spawn fspec bootstrap");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "bootstrap must exit 0; stderr={stderr}");

    // @step Then stdout contains the substring "# fspec Command - Kanban-Based Project Management"
    assert!(
        stdout.contains("# fspec Command - Kanban-Based Project Management"),
        "stdout must contain the header marker; got:\n{stdout}"
    );

    // @step Then stdout contains the substring "LIFECYCLE HOOKS"
    assert!(
        stdout.contains("LIFECYCLE HOOKS"),
        "stdout must contain LIFECYCLE HOOKS; got:\n{stdout}"
    );
}

#[test]
fn scenario_bootstrap_defines_no_positional_arguments_and_no_flags() {
    // @step Given the fspec Rust binary has been compiled

    // @step When I run `fspec bootstrap --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("bootstrap")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec bootstrap --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "bootstrap --help must exit 0");

    // @step Then the help output does not advertise any --skip-help, --minimal, or --skip-sections flags
    for forbidden in ["--skip-help", "--minimal", "--skip-sections"] {
        assert!(
            !stdout.contains(forbidden),
            "bootstrap must not advertise `{forbidden}`; got:\n{stdout}"
        );
    }
}

#[test]
fn scenario_default_combined_tui_mode_is_preserved_when_no_subcommand_is_provided() {
    // @step Given the fspec Rust binary has bootstrap registered as a clap subcommand alongside the existing subcommands

    // @step When I run `fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "fspec --help must exit 0");

    // @step Then the help output lists bootstrap as an available subcommand
    assert!(
        stdout.contains("bootstrap"),
        "top-level --help must list bootstrap; got:\n{stdout}"
    );
}

//! CLI surface for the `validate` subcommand on the standalone fspec Rust
//! binary — RPC-320.
//!
//! Feature: spec/features/validate-gherkin-cli-subcommand.feature
//!
//! PHASE B (TESTING): the clap subcommand is not yet wired into main.rs and
//! the core impl is still a NotYetPorted stub, so these tests are RED until
//! PHASE C + the supervisor's shared-file wiring pass. Each scenario maps 1:1
//! to a Gherkin scenario; @step comments mirror the step text verbatim.
//!
//! RPC-329 KNOWN DIVERGENCE: assertions cover structural facts and matching
//! substrings only — never the exact raw parser-error text.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ───────── helpers ─────────

fn run_validate(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("validate");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec validate");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_file(cwd: &Path, rel: &str, body: &str) {
    let path = cwd.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

const VALID_LOGIN: &str =
    "Feature: Login\n\n  Scenario: Valid login\n    Given I am on the login page\n    When I submit credentials\n    Then I see the dashboard\n";
const BROKEN: &str = "this is not gherkin";

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/validate.txt");

// ───────── scenarios ─────────

#[test]
fn cli_validates_a_single_valid_file_and_exits_0() {
    // Scenario: CLI validates a single valid file and exits 0

    // @step Given spec/features/login.feature is a syntactically valid feature file in the working directory
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/login.feature", VALID_LOGIN);

    // @step When I run `./rust/target/release/fspec validate spec/features/login.feature` from that directory
    let (code, stdout, stderr) = run_validate(ws.path(), &["spec/features/login.feature"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stdout={stdout}, stderr={stderr}");

    // @step Then stdout contains the substring '✓ spec/features/login.feature is valid'
    assert!(
        stdout.contains("✓ spec/features/login.feature is valid"),
        "stdout must report the valid file; got:\n{stdout}"
    );
}

#[test]
fn cli_exits_1_against_a_syntactically_broken_file() {
    // Scenario: CLI exits 1 against a syntactically broken file

    // @step Given spec/features/broken.feature contains broken Gherkin syntax in the working directory
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/broken.feature", BROKEN);

    // @step When I run `./rust/target/release/fspec validate spec/features/broken.feature` from that directory
    let (code, stdout, stderr) = run_validate(ws.path(), &["spec/features/broken.feature"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; stdout={stdout}, stderr={stderr}");

    // @step Then stdout contains the substring 'has syntax errors:'
    assert!(
        stdout.contains("has syntax errors:"),
        "stdout must mark the broken file; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Line '
    assert!(
        stdout.contains("Line "),
        "stdout must carry a 'Line ' detail; got:\n{stdout}"
    );
}

#[test]
fn cli_exits_2_when_no_feature_files_are_found() {
    // Scenario: CLI exits 2 when no feature files are found

    // @step Given spec/features/ exists but contains zero .feature files in the working directory
    let ws = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(ws.path().join("spec/features")).expect("mkdir spec/features");

    // @step When I run `./rust/target/release/fspec validate` from that directory
    let (code, stdout, stderr) = run_validate(ws.path(), &[]);

    // @step Then the command exits with code 2
    assert_eq!(code, 2, "must exit 2; stdout={stdout}, stderr={stderr}");

    // @step Then stderr contains the substring 'No feature files found in spec/features/'
    assert!(
        stderr.contains("No feature files found in spec/features/"),
        "stderr must carry the no-files message; got:\n{stderr}"
    );
}

#[test]
fn clap_exposes_validate_and_prints_help_byte_identical() {
    // Scenario: Clap exposes validate as a subcommand and prints help byte-identical to the TS reference

    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec validate --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("validate")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn validate --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "help must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the captured TS formatCommandHelp reference fixture
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step Then stdout contains the substring '--verbose'
    assert!(
        stdout.contains("--verbose"),
        "help must list the --verbose option; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'file'
    assert!(
        stdout.contains("file"),
        "help must mention the [file] argument; got:\n{stdout}"
    );
}

//! CLI surface for the `create-epic` subcommand on the standalone fspec
//! Rust binary — RPC-211.
//!
//! Feature: spec/features/create-epic-cli-subcommand.feature
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

fn run_create_epic(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("create-epic");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec create-epic");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_epics(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("epics.json"), raw).expect("write epics.json");
}

fn read_epics_value(cwd: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec/epics.json")).expect("read epics.json");
    serde_json::from_str(&raw).expect("parse epics.json")
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes create-epic with positional args and a --description flag in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_create_epic_with_args_and_description_flag() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec create-epic --help`
    let output = Command::new(fspec_bin())
        .arg("create-epic")
        .arg("--help")
        .output()
        .expect("spawn create-epic --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "create-epic --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout describes the create-epic subcommand
    assert!(
        stdout.contains("create-epic") || stdout.contains("CREATE-EPIC"),
        "help must describe create-epic; got:\n{stdout}"
    );

    // @step And stdout mentions the `<epicId>` argument
    assert!(
        stdout.contains("epicId"),
        "help must mention epicId; got:\n{stdout}"
    );

    // @step And stdout mentions the `<title>` argument
    assert!(
        stdout.contains("title"),
        "help must mention title; got:\n{stdout}"
    );

    // @step And stdout advertises the `--description` flag (or its `-d` short form)
    assert!(
        stdout.contains("--description") || stdout.contains("-d"),
        "help must advertise --description; got:\n{stdout}"
    );

    // @step And stdout does NOT advertise a `--workspace` global flag
    assert!(
        !stdout.contains("--workspace"),
        "create-epic --help must NOT advertise --workspace; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI creates a minimal epic and prints the success block
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_creates_minimal_epic_and_prints_success_block() {
    // @step Given an empty working directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec create-epic auth Authentication`
    let (code, stdout, stderr) = run_create_epic(ws.path(), &["auth", "Authentication"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Created epic auth'
    assert!(
        stdout.lines().any(|l| l == "✓ Created epic auth"),
        "missing checkmark line; got:\n{stdout}"
    );

    // @step And stdout contains the line '  Title: Authentication'
    assert!(
        stdout.lines().any(|l| l == "  Title: Authentication"),
        "missing title line; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring 'Description:'
    assert!(
        !stdout.contains("Description:"),
        "must omit Description; got:\n{stdout}"
    );

    // @step And the file spec/epics.json exists
    assert!(ws.path().join("spec/epics.json").exists());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI creates an epic with -d description and includes the Description line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_creates_epic_with_description_flag() {
    // @step Given an empty working directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./rust/target/release/fspec create-epic auth Authentication -d "Login features"`
    let (code, stdout, stderr) = run_create_epic(
        ws.path(),
        &["auth", "Authentication", "-d", "Login features"],
    );

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Created epic auth'
    assert!(
        stdout.lines().any(|l| l == "✓ Created epic auth"),
        "missing checkmark line; got:\n{stdout}"
    );

    // @step And stdout contains the line '  Title: Authentication'
    assert!(
        stdout.lines().any(|l| l == "  Title: Authentication"),
        "missing title line; got:\n{stdout}"
    );

    // @step And stdout contains the line '  Description: Login features'
    assert!(
        stdout.lines().any(|l| l == "  Description: Login features"),
        "missing description line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects an invalid epicId with exit 1 and stderr Error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_invalid_epic_id_with_exit_1() {
    // @step Given an empty working directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./rust/target/release/fspec create-epic INVALID Authentication`
    let (code, stdout, stderr) = run_create_epic(ws.path(), &["INVALID", "Authentication"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "expected exit 1 for invalid epicId; stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'lowercase-with-hyphens format'
    assert!(
        stderr.contains("lowercase-with-hyphens format"),
        "stderr must mention regex hint; got:\n{stderr}"
    );

    // @step And the file spec/epics.json does NOT exist
    assert!(
        !ws.path().join("spec/epics.json").exists(),
        "epics.json must not be written when validation fails"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects creating an epic that already exists with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_duplicate_epic_with_exit_1() {
    // @step Given spec/epics.json contains epic 'auth' with title='Old Title'
    let ws = tempfile::tempdir().expect("tempdir");
    write_epics(
        ws.path(),
        r#"{
  "epics": {
    "auth": {
      "id": "auth",
      "title": "Old Title",
      "createdAt": "2026-06-01T00:00:00.000Z"
    }
  }
}"#,
    );

    // @step When I run `./rust/target/release/fspec create-epic auth NewTitle`
    let (code, stdout, stderr) = run_create_epic(ws.path(), &["auth", "NewTitle"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "expected exit 1 for duplicate; stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Epic auth already exists'
    assert!(
        stderr.contains("Epic auth already exists"),
        "stderr must report duplicate; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given an empty working directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I dispatch create-epic via fspec_core::dispatch::dispatch_command with epicId='auth' title='Authentication'
    let req = codelet_fspec_core::DispatchRequest {
        command: "create-epic".to_string(),
        args_json: r#"{"epicId":"auth","title":"Authentication"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step Then the dispatcher writes spec/epics.json
    assert!(ws.path().join("spec/epics.json").exists());

    // @step And running `./rust/target/release/fspec create-epic dash Dashboard` afterwards exits 0
    let (code, stdout, stderr) = run_create_epic(ws.path(), &["dash", "Dashboard"]);
    assert_eq!(
        code, 0,
        "CLI add must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/epics.json now contains both 'auth' and 'dash' epics
    let data = read_epics_value(ws.path());
    assert!(data["epics"].get("auth").is_some(), "auth must be present");
    assert!(data["epics"].get("dash").is_some(), "dash must be present");

    // @step And the CLI bridge module rust/fspec/src/create_epic.rs contains NO inline epic-id validation, duplicate-check, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/create_epic.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/create_epic.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    // The forbidden tokens here are domain-logic markers — patterns that
    // would only appear if the bridge duplicated the core's id-format
    // validator, epic table, write helper, or success rendering. The
    // TS-parity stderr text `Error: <reason>` is the CLI surface and
    // lives in the bridge; we don't forbid the verb-phrase that the
    // core wraps duplicate-/IO-failure errors with because mentioning
    // it in a rustdoc comment isn't a duplication risk. The dispatch
    // path is enforced by the `read_epics_or_empty` / `write_json_atomic`
    // checks below.
    for forbidden in [
        "EPIC_ID_REGEX",
        "[a-z][a-z0-9]",
        "already exists",
        "✓ Created epic",
        "write_json_atomic",
        "read_epics_or_empty",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: create-epic --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_CE: &str = include_str!("fixtures/help/create-epic.txt");

#[test]
fn scenario_create_epic_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec create-epic --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("create-epic")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn create-epic --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "create-epic --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/create-epic.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_CE);

    // @step And stdout starts with a blank line followed by 'CREATE-EPIC'
    assert!(stdout.starts_with("\nCREATE-EPIC\n"));
}

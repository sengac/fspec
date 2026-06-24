//! CLI surface for the `add-architecture-note` subcommand on the standalone
//! fspec Rust binary — RPC-168.
//!
//! Feature: spec/features/add-architecture-note-cli-subcommand.feature
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

fn run_add_arch(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-architecture-note");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-architecture-note");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units_value(cwd: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec/work-units.json")).expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn seed_one_unit(cwd: &Path) {
    write_work_units(
        cwd,
        r#"{
  "version": "0.7.1",
  "meta": {"version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z"},
  "workUnits": {
    "AUTH-001": {
      "id": "AUTH-001",
      "title": "Login",
      "status": "specifying",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }
  },
  "states": {
    "backlog": [], "specifying": ["AUTH-001"], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes add-architecture-note with positional args in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_add_architecture_note_with_positional_args_in_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec add-architecture-note --help`
    let output = Command::new(fspec_bin())
        .arg("add-architecture-note")
        .arg("--help")
        .output()
        .expect("spawn add-architecture-note --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "add-architecture-note --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout describes the add-architecture-note subcommand
    assert!(
        stdout.contains("add-architecture-note") || stdout.contains("ADD-ARCHITECTURE-NOTE"),
        "help must describe add-architecture-note; got:\n{stdout}"
    );

    // @step And stdout mentions the `<workUnitId>` argument
    assert!(
        stdout.contains("workUnitId"),
        "help must mention workUnitId; got:\n{stdout}"
    );

    // @step And stdout mentions the `<note>` argument
    assert!(
        stdout.contains("note"),
        "help must mention note; got:\n{stdout}"
    );

    // @step And stdout does NOT advertise a `--workspace` global flag
    assert!(
        !stdout.contains("--workspace"),
        "add-architecture-note --help must NOT advertise --workspace; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI appends an architecture note and prints the success block
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_appends_architecture_note_and_prints_success_block() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' with no architectureNotes
    let ws = tempfile::tempdir().expect("tempdir");
    seed_one_unit(ws.path());

    // @step When I run `./codelet/target/release/fspec add-architecture-note AUTH-001 "Uses bcrypt"`
    let (code, stdout, stderr) = run_add_arch(ws.path(), &["AUTH-001", "Uses bcrypt"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ Architecture note added successfully'
    assert!(
        stdout
            .lines()
            .any(|l| l == "✓ Architecture note added successfully"),
        "missing checkmark line; got:\n{stdout}"
    );

    // @step And stdout contains the substring '<system-reminder>'
    assert!(
        stdout.contains("<system-reminder>"),
        "missing system-reminder block; got:\n{stdout}"
    );

    // @step And spec/work-units.json work unit 'AUTH-001' has one architectureNote with text='Uses bcrypt'
    let data = read_work_units_value(ws.path());
    let notes = data["workUnits"]["AUTH-001"]["architectureNotes"]
        .as_array()
        .expect("notes array");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0]["text"].as_str(), Some("Uses bcrypt"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects an unknown work unit with exit 1 and stderr Error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_unknown_work_unit_with_exit_1() {
    // @step Given spec/work-units.json contains no work unit 'MISSING-001'
    let ws = tempfile::tempdir().expect("tempdir");
    seed_one_unit(ws.path());

    // @step When I run `./codelet/target/release/fspec add-architecture-note MISSING-001 "any note"`
    let (code, stdout, stderr) = run_add_arch(ws.path(), &["MISSING-001", "any note"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Work unit 'MISSING-001' does not exist"
    assert!(
        stderr.contains("Work unit 'MISSING-001' does not exist"),
        "stderr must mention missing work unit; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001'
    let ws = tempfile::tempdir().expect("tempdir");
    seed_one_unit(ws.path());

    // @step When I dispatch add-architecture-note via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' note='via dispatcher'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-architecture-note".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","note":"via dispatcher"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step Then the dispatcher writes spec/work-units.json
    assert!(ws.path().join("spec/work-units.json").exists());

    // @step And running `./codelet/target/release/fspec add-architecture-note AUTH-001 "via cli"` afterwards exits 0
    let (code, stdout, stderr) = run_add_arch(ws.path(), &["AUTH-001", "via cli"]);
    assert_eq!(
        code, 0,
        "CLI add must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/work-units.json work unit 'AUTH-001' contains two architectureNotes
    let data = read_work_units_value(ws.path());
    let notes = data["workUnits"]["AUTH-001"]["architectureNotes"]
        .as_array()
        .expect("notes array");
    assert_eq!(notes.len(), 2, "expected two notes, got {notes:?}");

    // @step And the CLI bridge module codelet/fspec/src/add_architecture_note.rs contains NO inline note-append, nextNoteId, or system-reminder rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_architecture_note.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/add_architecture_note.rs must exist; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "nextNoteId",
        "ARCHITECTURE NOTE ADDED",
        "architectureNotes",
        "write_json_atomic",
        "ensure_work_units_file",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: add-architecture-note --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/add-architecture-note.txt");

#[test]
fn scenario_add_architecture_note_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec add-architecture-note --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("add-architecture-note")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-architecture-note --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "add-architecture-note --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/add-architecture-note.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step And stdout starts with a blank line followed by 'ADD-ARCHITECTURE-NOTE'
    assert!(stdout.starts_with("\nADD-ARCHITECTURE-NOTE\n"));
}

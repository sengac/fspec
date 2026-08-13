//! CLI surface for the `restore-architecture-note` subcommand on the standalone
//! fspec Rust binary — RPC-287.
//!
//! Feature: spec/features/restore-architecture-note-cli-subcommand.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_restore_arch(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("restore-architecture-note");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec restore-architecture-note");
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

fn seed_unit_with_notes(cwd: &Path, notes: &str) {
    let body = format!(
        r#"{{
  "version": "0.7.1",
  "meta": {{"version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z"}},
  "workUnits": {{
    "AUTH-001": {{
      "id": "AUTH-001",
      "title": "Login",
      "status": "specifying",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z",
      "architectureNotes": {notes}
    }}
  }},
  "states": {{
    "backlog": [], "specifying": ["AUTH-001"], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }}
}}"#
    );
    write_work_units(cwd, &body);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes restore-architecture-note with two positional args in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_restore_architecture_note_with_two_positional_args_in_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec restore-architecture-note --help`
    let output = Command::new(fspec_bin())
        .arg("restore-architecture-note")
        .arg("--help")
        .output()
        .expect("spawn restore-architecture-note --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "restore-architecture-note --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout describes the restore-architecture-note subcommand
    assert!(
        stdout.contains("restore-architecture-note")
            || stdout.contains("RESTORE-ARCHITECTURE-NOTE"),
        "help must describe restore-architecture-note; got:\n{stdout}"
    );

    // @step And stdout mentions the `<workUnitId>` argument
    assert!(
        stdout.contains("workUnitId"),
        "help must mention workUnitId; got:\n{stdout}"
    );

    // @step And stdout mentions the `<index>` argument
    assert!(
        stdout.contains("index"),
        "help must mention index; got:\n{stdout}"
    );

    // @step And stdout does NOT advertise a `--workspace` global flag
    assert!(
        !stdout.contains("--workspace"),
        "restore-architecture-note --help must NOT advertise --workspace; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI restores an architecture note and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_restores_architecture_note_and_prints_success_line() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' with one architectureNote id=0 text 'Note A' marked deleted
    let ws = tempfile::tempdir().expect("tempdir");
    seed_unit_with_notes(
        ws.path(),
        r#"[{ "id": 0, "text": "Note A", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "2026-06-01T00:00:00.000Z" }]"#,
    );

    // @step When I run `./rust/target/release/fspec restore-architecture-note AUTH-001 0`
    let (code, stdout, stderr) = run_restore_arch(ws.path(), &["AUTH-001", "0"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "must exit 0; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stdout contains the line '✓ Architecture note restored successfully'
    assert!(
        stdout.contains("✓ Architecture note restored successfully"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints idempotent message when already active
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_idempotent_message_when_already_active() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' with one architectureNote id=0 text 'Note A' deleted=false
    let ws = tempfile::tempdir().expect("tempdir");
    seed_unit_with_notes(
        ws.path(),
        r#"[{ "id": 0, "text": "Note A", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z" }]"#,
    );

    // @step When I run `./rust/target/release/fspec restore-architecture-note AUTH-001 0`
    let (code, stdout, stderr) = run_restore_arch(ws.path(), &["AUTH-001", "0"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "must exit 0; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stdout contains the line '✓ Architecture note restored successfully'
    assert!(
        stdout.contains("✓ Architecture note restored successfully"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Item ID 0 already active'
    assert!(
        stdout.contains("Item ID 0 already active"),
        "stdout must contain idempotent message; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects unknown note ID with exit 1 and stderr Error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_unknown_note_id_with_exit_1_and_error_prefix() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' with one architectureNote id=0 marked deleted
    let ws = tempfile::tempdir().expect("tempdir");
    seed_unit_with_notes(
        ws.path(),
        r#"[{ "id": 0, "text": "N", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "x" }]"#,
    );

    // @step When I run `./rust/target/release/fspec restore-architecture-note AUTH-001 5`
    let (code, stdout, stderr) = run_restore_arch(ws.path(), &["AUTH-001", "5"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Architecture note with ID 5 not found'
    assert!(
        stderr.contains("Architecture note with ID 5 not found"),
        "stderr must mention ID not found; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects unknown work unit with exit 1 and stderr Error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_unknown_work_unit_with_exit_1_and_error_prefix() {
    // @step Given spec/work-units.json contains no work unit 'AUTH-999'
    let ws = tempfile::tempdir().expect("tempdir");
    seed_unit_with_notes(
        ws.path(),
        r#"[{ "id": 0, "text": "N", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "x" }]"#,
    );

    // @step When I run `./rust/target/release/fspec restore-architecture-note AUTH-999 0`
    let (code, stdout, stderr) = run_restore_arch(ws.path(), &["AUTH-999", "0"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Work unit'
    assert!(
        stderr.contains("Work unit"),
        "stderr must mention Work unit; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    use codelet_fspec_core::{dispatch_command, DispatchRequest};

    // @step Given spec/work-units.json contains work unit 'AUTH-001' with two architectureNotes id=0 and id=1 both marked deleted
    let ws = tempfile::tempdir().expect("tempdir");
    seed_unit_with_notes(
        ws.path(),
        r#"[
        { "id": 0, "text": "N0", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "x" },
        { "id": 1, "text": "N1", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "x" }
      ]"#,
    );

    // @step When I dispatch restore-architecture-note via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' index=0
    let result = dispatch_command(DispatchRequest {
        command: "restore-architecture-note".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","index":0}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    });
    assert!(result.success, "dispatcher must succeed: {result:?}");

    // @step Then the dispatcher mutates spec/work-units.json
    let on_disk = read_work_units_value(ws.path());
    let n0 = on_disk["workUnits"]["AUTH-001"]["architectureNotes"]
        .as_array()
        .expect("notes array")
        .iter()
        .find(|n| n["id"].as_u64() == Some(0))
        .expect("id=0 exists");
    assert_eq!(n0["deleted"].as_bool(), Some(false));

    // @step And running `./rust/target/release/fspec restore-architecture-note AUTH-001 1` afterwards exits 0
    let (code, stdout, stderr) = run_restore_arch(ws.path(), &["AUTH-001", "1"]);
    assert_eq!(
        code, 0,
        "second CLI call must exit 0; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/work-units.json on disk shows both architectureNotes with deleted=false
    let on_disk = read_work_units_value(ws.path());
    let arr = on_disk["workUnits"]["AUTH-001"]["architectureNotes"]
        .as_array()
        .expect("notes array");
    assert_eq!(arr.len(), 2);
    for n in arr {
        assert_eq!(
            n["deleted"].as_bool(),
            Some(false),
            "all notes active; got {n}"
        );
    }

    // @step And the CLI bridge module rust/fspec/src/restore_architecture_note.rs contains NO inline state mutation or file-write logic — its only computation is JSON arg marshalling
    let bridge = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/restore_architecture_note.rs"),
    )
    .expect("read restore_architecture_note.rs bridge");
    for forbidden in &["write_json_atomic", "ensure_work_units_file"] {
        assert!(
            !bridge.contains(forbidden),
            "CLI bridge must not contain '{forbidden}'; got bridge module:\n{bridge}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: restore-architecture-note --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_RAN: &str = include_str!("fixtures/help/restore-architecture-note.txt");

#[test]
fn scenario_restore_architecture_note_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec restore-architecture-note --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("restore-architecture-note")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn restore-architecture-note --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "restore-architecture-note --help must exit 0");

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/restore-architecture-note.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_RAN);

    // @step And stdout starts with a blank line followed by 'RESTORE-ARCHITECTURE-NOTE'
    assert!(
        stdout.starts_with("\nRESTORE-ARCHITECTURE-NOTE"),
        "stdout must start with blank line + RESTORE-ARCHITECTURE-NOTE; got:\n{stdout}"
    );
}

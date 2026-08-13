//! CLI surface for the `restore-question` subcommand on the standalone fspec
//! Rust binary — RPC-290.
//!
//! Feature: spec/features/restore-question-cli-subcommand.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

fn run_restore_question(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("restore-question");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec restore-question");
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

fn read_work_units(cwd: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec/work-units.json")).expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn work_units_specifying_with_questions(extras_json: &str) -> String {
    format!(
        r#"{{
  "version": "0.7.1",
  "meta": {{ "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" }},
  "workUnits": {{
    "AUTH-001": {{
      "id": "AUTH-001",
      "title": "Test",
      "status": "specifying",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z",
      {extras_json}
    }}
  }},
  "states": {{
    "backlog": [], "specifying": ["AUTH-001"], "testing": [], "implementing": [],
    "validating": [], "done": [], "blocked": []
  }}
}}"#,
        extras_json = extras_json.trim(),
    )
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes restore-question with two positional args in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_restore_question_with_two_positional_args_in_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec restore-question --help`
    let output = Command::new(fspec_bin())
        .arg("restore-question")
        .arg("--help")
        .output()
        .expect("spawn restore-question --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "restore-question --help must exit 0; stderr={stderr}"
    );

    // @step And stdout describes the restore-question subcommand
    assert!(
        stdout.contains("restore-question") || stdout.contains("RESTORE-QUESTION"),
        "help must describe restore-question; got:\n{stdout}"
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
        "restore-question --help must NOT advertise --workspace; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI restores a question and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_restores_question_and_prints_success_line() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 text 'Q?' marked deleted
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = work_units_specifying_with_questions(
        r#""questions": [{ "id": 0, "text": "Q?", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "2026-06-01T00:00:00.000Z", "selected": false }], "nextQuestionId": 1"#,
    );
    write_work_units(ws.path(), &raw);

    // @step When I run `./rust/target/release/fspec restore-question AUTH-001 0`
    let (code, stdout, stderr) = run_restore_question(ws.path(), &["AUTH-001", "0"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "must exit 0; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stdout contains the line '✓ Restored question: "Q?"'
    assert!(
        stdout.contains("✓ Restored question: \"Q?\""),
        "stdout must contain canonical restored line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints idempotent success message when already active
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_idempotent_success_message_when_already_active() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 text 'Q?' deleted=false
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = work_units_specifying_with_questions(
        r#""questions": [{ "id": 0, "text": "Q?", "deleted": false, "createdAt": "2026-06-01T00:00:00.000Z", "selected": false }], "nextQuestionId": 1"#,
    );
    write_work_units(ws.path(), &raw);

    // @step When I run `./rust/target/release/fspec restore-question AUTH-001 0`
    let (code, stdout, stderr) = run_restore_question(ws.path(), &["AUTH-001", "0"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "must exit 0; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stdout contains the line '✓ Restored question: "Q?"'
    assert!(
        stdout.contains("✓ Restored question: \"Q?\""),
        "stdout must contain canonical restored line; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Item ID 0 already active'
    assert!(
        stdout.contains("Item ID 0 already active"),
        "stdout must contain idempotent message; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects unknown question ID with exit 1 and stderr Failed prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_unknown_question_id_with_exit_1_and_error_prefix() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with one question id=0 marked deleted
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = work_units_specifying_with_questions(
        r#""questions": [{ "id": 0, "text": "Q?", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "x", "selected": false }]"#,
    );
    write_work_units(ws.path(), &raw);

    // @step When I run `./rust/target/release/fspec restore-question AUTH-001 5`
    let (code, stdout, stderr) = run_restore_question(ws.path(), &["AUTH-001", "5"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Question with ID 5 not found'
    assert!(
        stderr.contains("Question with ID 5 not found"),
        "stderr must mention ID not found; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects unknown work unit with exit 1 and stderr Failed prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_unknown_work_unit_with_exit_1_and_error_prefix() {
    // @step Given spec/work-units.json contains no work unit 'AUTH-999'
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = work_units_specifying_with_questions(
        r#""questions": [{ "id": 0, "text": "Q?", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "x", "selected": false }]"#,
    );
    write_work_units(ws.path(), &raw);

    // @step When I run `./rust/target/release/fspec restore-question AUTH-999 0`
    let (code, stdout, stderr) = run_restore_question(ws.path(), &["AUTH-999", "0"]);

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

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status with two questions id=0 and id=1 both marked deleted
    let ws = tempfile::tempdir().expect("tempdir");
    let raw = work_units_specifying_with_questions(
        r#""questions": [
        { "id": 0, "text": "Q0", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "x", "selected": false },
        { "id": 1, "text": "Q1", "deleted": true, "deletedAt": "1999-01-01T00:00:00.000Z", "createdAt": "x", "selected": false }
      ], "nextQuestionId": 2"#,
    );
    write_work_units(ws.path(), &raw);

    // @step When I dispatch restore-question via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' index=0
    let result = dispatch_command(DispatchRequest {
        command: "restore-question".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","index":0}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    });
    assert!(result.success, "dispatcher must succeed: {result:?}");

    // @step Then the dispatcher mutates spec/work-units.json
    let on_disk = read_work_units(ws.path());
    let q0 = on_disk["workUnits"]["AUTH-001"]["questions"]
        .as_array()
        .expect("questions array")
        .iter()
        .find(|q| q["id"].as_u64() == Some(0))
        .expect("id=0 exists");
    assert_eq!(q0["deleted"].as_bool(), Some(false));

    // @step And running `./rust/target/release/fspec restore-question AUTH-001 1` afterwards exits 0
    let (code, stdout, stderr) = run_restore_question(ws.path(), &["AUTH-001", "1"]);
    assert_eq!(
        code, 0,
        "second CLI call must exit 0; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/work-units.json on disk shows both questions with deleted=false
    let on_disk = read_work_units(ws.path());
    let arr = on_disk["workUnits"]["AUTH-001"]["questions"]
        .as_array()
        .expect("questions array");
    assert_eq!(arr.len(), 2);
    for q in arr {
        assert_eq!(
            q["deleted"].as_bool(),
            Some(false),
            "all questions active; got {q}"
        );
    }

    // @step And the CLI bridge module rust/fspec/src/restore_question.rs contains NO inline state mutation or file-write logic — its only computation is JSON arg marshalling
    let bridge = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/restore_question.rs"),
    )
    .expect("read restore_question.rs bridge");
    for forbidden in &["write_json_atomic", "ensure_work_units_file"] {
        assert!(
            !bridge.contains(forbidden),
            "CLI bridge must not contain '{forbidden}'; got bridge module:\n{bridge}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: restore-question --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_RQ: &str = include_str!("fixtures/help/restore-question.txt");

#[test]
fn scenario_restore_question_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec restore-question --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("restore-question")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn restore-question --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "restore-question --help must exit 0");

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/restore-question.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_RQ);

    // @step And stdout starts with a blank line followed by 'RESTORE-QUESTION'
    assert!(
        stdout.starts_with("\nRESTORE-QUESTION"),
        "stdout must start with blank line + RESTORE-QUESTION; got:\n{stdout}"
    );
}

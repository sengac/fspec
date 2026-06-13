//! CLI surface for the `add-question` subcommand on the standalone fspec
//! Rust binary — RPC-188.
//!
//! Feature: spec/features/add-question-cli-subcommand.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

fn run_add_question(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-question");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-question");
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
    let raw = fs::read_to_string(cwd.join("spec/work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn minimal_work_units(id: &str, status: &str) -> String {
    // Each status bucket is rendered exactly once below, with the work
    // unit placed into the bucket matching `status`. This avoids any
    // duplicate JSON keys in the `states` object.
    let id_array = format!("[\"{id}\"]");
    let bucket = id_array.as_str();
    let (backlog, specifying, testing, implementing, validating, done, blocked) = match status {
        "backlog" => (bucket, "[]", "[]", "[]", "[]", "[]", "[]"),
        "specifying" => ("[]", bucket, "[]", "[]", "[]", "[]", "[]"),
        "testing" => ("[]", "[]", bucket, "[]", "[]", "[]", "[]"),
        "implementing" => ("[]", "[]", "[]", bucket, "[]", "[]", "[]"),
        "validating" => ("[]", "[]", "[]", "[]", bucket, "[]", "[]"),
        "done" => ("[]", "[]", "[]", "[]", "[]", bucket, "[]"),
        "blocked" => ("[]", "[]", "[]", "[]", "[]", "[]", bucket),
        _ => ("[]", "[]", "[]", "[]", "[]", "[]", "[]"),
    };
    format!(
        r#"{{
  "version": "0.7.1",
  "meta": {{ "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" }},
  "workUnits": {{
    "{id}": {{
      "id": "{id}",
      "title": "Test",
      "status": "{status}",
      "createdAt": "2026-06-01T00:00:00.000Z",
      "updatedAt": "2026-06-01T00:00:00.000Z"
    }}
  }},
  "states": {{
    "backlog": {backlog}, "specifying": {specifying}, "testing": {testing},
    "implementing": {implementing}, "validating": {validating}, "done": {done},
    "blocked": {blocked}
  }}
}}"#
    )
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes add-question with two positional args in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_add_question_with_two_positional_args_in_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec.)

    // @step When I run `./codelet/target/release/fspec add-question --help`
    let output = Command::new(fspec_bin())
        .arg("add-question")
        .arg("--help")
        .output()
        .expect("spawn add-question --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "add-question --help must exit 0; stderr={stderr}");

    // @step And stdout describes the add-question subcommand
    assert!(
        stdout.contains("add-question") || stdout.contains("ADD-QUESTION"),
        "help must describe add-question; got:\n{stdout}"
    );

    // @step And stdout mentions the `<workUnitId>` argument
    assert!(
        stdout.contains("workUnitId"),
        "help must mention workUnitId; got:\n{stdout}"
    );

    // @step And stdout mentions the `<question>` argument
    assert!(
        stdout.contains("question"),
        "help must mention question; got:\n{stdout}"
    );

    // @step And stdout does NOT advertise a `--workspace` global flag
    assert!(
        !stdout.contains("--workspace"),
        "add-question --help must NOT advertise --workspace; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI adds a question and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_adds_question_and_prints_success_line() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &minimal_work_units("AUTH-001", "specifying"));

    // @step When I run `./codelet/target/release/fspec add-question AUTH-001 "Should we add OAuth?"`
    let (code, stdout, stderr) =
        run_add_question(ws.path(), &["AUTH-001", "Should we add OAuth?"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "add-question must exit 0; got {code}, stderr={stderr}, stdout={stdout}"
    );

    // @step And stdout contains the line '✓ Question added successfully'
    assert!(
        stdout.contains("✓ Question added successfully"),
        "stdout must contain success line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects unknown work unit with exit 1 and stderr Failed prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_unknown_work_unit_with_exit_1_and_error_prefix() {
    // @step Given spec/work-units.json contains no work unit 'AUTH-999'
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &minimal_work_units("AUTH-001", "specifying"));

    // @step When I run `./codelet/target/release/fspec add-question AUTH-999 "Q?"`
    let (code, stdout, stderr) = run_add_question(ws.path(), &["AUTH-999", "Q?"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; got {code}, stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to add question:'
    assert!(
        stderr.contains("✗ Failed to add question:"),
        "stderr must contain TS-parity Failed-to-add-question prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Work unit'
    assert!(
        stderr.contains("Work unit"),
        "stderr must contain Work unit message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects wrong status with exit 1 and stderr Failed prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_wrong_status_with_exit_1_and_error_prefix() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'backlog' status
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &minimal_work_units("AUTH-001", "backlog"));

    // @step When I run `./codelet/target/release/fspec add-question AUTH-001 "Q?"`
    let (code, stdout, stderr) = run_add_question(ws.path(), &["AUTH-001", "Q?"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; got {code}, stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to add question:'
    assert!(
        stderr.contains("✗ Failed to add question:"),
        "stderr must contain TS-parity Failed-to-add-question prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'discovery/specification phase'
    assert!(
        stderr.contains("discovery/specification phase"),
        "stderr must contain phase guard message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    use codelet_fspec_core::{dispatch_command, DispatchRequest};

    // @step Given spec/work-units.json contains work unit 'AUTH-001' in 'specifying' status
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &minimal_work_units("AUTH-001", "specifying"));

    // @step When I dispatch add-question via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' question='dispatched'
    let result = dispatch_command(DispatchRequest {
        command: "add-question".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","question":"dispatched"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    });
    assert!(result.success, "dispatcher must succeed: {result:?}");

    // @step Then the dispatcher mutates spec/work-units.json
    let on_disk = read_work_units(ws.path());
    let questions = on_disk["workUnits"]["AUTH-001"]["questions"]
        .as_array()
        .expect("questions array");
    assert_eq!(questions.len(), 1);
    assert_eq!(questions[0]["text"].as_str(), Some("dispatched"));

    // @step And running `./codelet/target/release/fspec add-question AUTH-001 "from-cli"` afterwards exits 0
    let (code, stdout, stderr) = run_add_question(ws.path(), &["AUTH-001", "from-cli"]);
    assert_eq!(
        code, 0,
        "second CLI call must exit 0; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/work-units.json now contains both 'dispatched' and 'from-cli' question texts on AUTH-001
    let on_disk = read_work_units(ws.path());
    let texts: Vec<&str> = on_disk["workUnits"]["AUTH-001"]["questions"]
        .as_array()
        .expect("questions array")
        .iter()
        .filter_map(|q| q["text"].as_str())
        .collect();
    assert!(
        texts.contains(&"dispatched"),
        "questions must include 'dispatched'; got {texts:?}"
    );
    assert!(
        texts.contains(&"from-cli"),
        "questions must include 'from-cli'; got {texts:?}"
    );

    // @step And the CLI bridge module codelet/fspec/src/add_question.rs contains NO inline state mutation or file-write logic — its only computation is JSON arg marshalling
    let bridge = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_question.rs"),
    )
    .expect("read add_question.rs bridge");
    for forbidden in &["write_json_atomic", "ensure_work_units_file"] {
        assert!(
            !bridge.contains(forbidden),
            "CLI bridge must not contain '{forbidden}'; got bridge module:\n{bridge}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: add-question --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_AQ: &str = include_str!("fixtures/help/add-question.txt");

#[test]
fn scenario_add_question_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec add-question --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("add-question")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-question --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "add-question --help must exit 0");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/add-question.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_AQ);

    // @step And stdout starts with a blank line followed by 'ADD-QUESTION'
    assert!(
        stdout.starts_with("\nADD-QUESTION"),
        "stdout must start with blank line + ADD-QUESTION; got:\n{stdout}"
    );
}

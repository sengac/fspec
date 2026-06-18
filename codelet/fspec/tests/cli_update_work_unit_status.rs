//! CLI surface for the `update-work-unit-status` subcommand on the standalone
//! fspec Rust binary — RPC-319.
//!
//! Feature: spec/features/update-work-unit-status-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario; @step comments mirror the
//! Gherkin step text verbatim.
//!
//! RED PHASE (Phase B): the `update-work-unit-status` clap subcommand is not
//! wired until Phase C and the core impl is still the 1-arg NotYetPorted stub,
//! so these tests are EXPECTED to fail until then.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ---------- helpers ----------

fn run_uwus(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("update-work-unit-status");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec update-work-unit-status");
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

fn write_feature(cwd: &Path, name: &str, id: &str, body: &str) {
    let dir = cwd.join("spec").join("features");
    fs::create_dir_all(&dir).expect("mkdir features");
    let content = format!("@{id}\nFeature: {name}\n\n{body}\n");
    fs::write(dir.join(format!("{name}.feature")), content).expect("write feature");
}

/// Single-unit work-units.json placing `id` in the `status` state array.
fn doc(id: &str, status: &str, extra_fields: &str) -> String {
    let mut parts = Vec::new();
    for s in [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        if s == status {
            parts.push(format!(r#""{s}": ["{id}"]"#));
        } else {
            parts.push(format!(r#""{s}": []"#));
        }
    }
    let states = parts.join(", ");
    let extra = if extra_fields.trim().is_empty() {
        String::new()
    } else {
        format!(", {extra_fields}")
    };
    format!(
        r#"{{
  "version": "0.7.1",
  "meta": {{ "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" }},
  "workUnits": {{
    "{id}": {{
      "id": "{id}", "title": "Login", "type": "story", "status": "{status}",
      "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"{extra}
    }}
  }},
  "states": {{ {states} }}
}}"#
    )
}

// ---------- scenarios ----------

#[test]
fn scenario_cli_applies_valid_transition_and_exits_zero() {
    // @step Given a work unit "AUTH-001" exists with status "backlog"
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &doc("AUTH-001", "backlog", ""));

    // @step When I run `fspec update-work-unit-status AUTH-001 specifying`
    let (code, stdout, stderr) = run_uwus(ws.path(), &["AUTH-001", "specifying"]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stdout={stdout}, stderr={stderr}");

    // @step And stdout confirms the status changed to "specifying"
    assert!(
        stdout.contains("specifying") && stdout.contains("AUTH-001"),
        "stdout must confirm status change to specifying; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_rejects_invalid_transition_with_nonzero_exit() {
    // @step Given a work unit "AUTH-001" exists with status "backlog"
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &doc("AUTH-001", "backlog", ""));

    // @step When I run `fspec update-work-unit-status AUTH-001 done`
    let (code, stdout, stderr) = run_uwus(ws.path(), &["AUTH-001", "done"]);

    // @step Then the command exits with a non-zero code
    assert_ne!(code, 0, "expected non-zero exit; stdout={stdout}");

    // @step And stderr names the allowed transitions from "backlog"
    assert!(
        stderr.contains("Invalid state transition from 'backlog'") || stderr.contains("backlog"),
        "stderr must name allowed transitions from backlog; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_rejects_unknown_work_unit_id() {
    // @step Given no work unit "NOPE-999" exists
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &doc("AUTH-001", "backlog", ""));

    // @step When I run `fspec update-work-unit-status NOPE-999 specifying`
    let (code, _stdout, stderr) = run_uwus(ws.path(), &["NOPE-999", "specifying"]);

    // @step Then the command exits with a non-zero code
    assert_ne!(code, 0, "expected non-zero exit");

    // @step And stderr contains "Work unit NOPE-999 does not exist"
    assert!(
        stderr.contains("Work unit NOPE-999 does not exist")
            || stderr.contains("Work unit 'NOPE-999' does not exist"),
        "stderr must contain not-found text; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_requires_blocked_reason_when_moving_to_blocked() {
    // @step Given a work unit "AUTH-001" exists with status "specifying"
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &doc("AUTH-001", "specifying", ""));

    // @step When I run `fspec update-work-unit-status AUTH-001 blocked`
    let (code, _stdout, stderr) = run_uwus(ws.path(), &["AUTH-001", "blocked"]);

    // @step Then the command exits with a non-zero code
    assert_ne!(code, 0, "expected non-zero exit without --blocked-reason");

    // @step And stderr requires a blocked reason
    assert!(
        stderr.to_lowercase().contains("blocked reason")
            || stderr.contains("blocked-reason")
            || stderr.contains("blockedReason"),
        "stderr must require a blocked reason; got:\n{stderr}"
    );

    // @step When I run `fspec update-work-unit-status AUTH-001 blocked --blocked-reason "waiting on API"`
    let (code2, stdout2, stderr2) = run_uwus(
        ws.path(),
        &["AUTH-001", "blocked", "--blocked-reason", "waiting on API"],
    );

    // @step Then the command exits with code 0
    assert_eq!(code2, 0, "expected exit 0; stderr={stderr2}");

    // @step And stdout confirms the status changed to "blocked"
    assert!(
        stdout2.contains("blocked") && stdout2.contains("AUTH-001"),
        "stdout must confirm status change to blocked; got:\n{stdout2}"
    );
}

#[test]
fn scenario_cli_rejects_unknown_status_value() {
    // @step Given a work unit "AUTH-001" exists with status "backlog"
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &doc("AUTH-001", "backlog", ""));

    // @step When I run `fspec update-work-unit-status AUTH-001 frobnicate`
    let (code, _stdout, stderr) = run_uwus(ws.path(), &["AUTH-001", "frobnicate"]);

    // @step Then the command exits with a non-zero code
    assert_ne!(code, 0, "expected non-zero exit for unknown status");

    // @step And stderr reports that the status must be one of the allowed states
    assert!(
        stderr.contains("Invalid status value") || stderr.contains("Allowed values"),
        "stderr must report allowed states; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_honours_skip_temporal_validation() {
    // @step Given a work unit "AUTH-001" exists with status "specifying"
    let ws = tempfile::tempdir().expect("tempdir");
    // stateHistory has a future specifying entry so the feature file (written
    // now) violates temporal ordering unless skipped. The work unit also
    // satisfies review validation (rules + examples + architecture notes + AST
    // research attachment) so the temporal gate is actually the one that fires.
    write_work_units(
        ws.path(),
        &doc(
            "AUTH-001",
            "specifying",
            r#""stateHistory": [{ "state": "specifying", "timestamp": "2999-01-01T00:00:00.000Z" }], "rules": [{ "id": 0, "text": "rule", "deleted": false }], "examples": [{ "id": 0, "text": "example", "deleted": false }], "architectureNotes": [{ "id": 0, "text": "note", "deleted": false }], "attachments": ["spec/attachments/AUTH-001/ast-research-login.json"]"#,
        ),
    );

    // @step And its linked feature file was last modified before the work unit entered "specifying"
    write_feature(
        ws.path(),
        "user-login",
        "AUTH-001",
        "Scenario: Login with valid credentials\n    Given I am on the login page\n    When I enter valid credentials\n    Then I should see the dashboard",
    );

    // @step When I run `fspec update-work-unit-status AUTH-001 testing`
    let (code, _stdout, _stderr) = run_uwus(ws.path(), &["AUTH-001", "testing"]);

    // @step Then the command exits with a non-zero code
    assert_ne!(code, 0, "expected non-zero exit due to temporal violation");

    // @step When I run `fspec update-work-unit-status AUTH-001 testing --skip-temporal-validation`
    let (code2, stdout2, stderr2) =
        run_uwus(ws.path(), &["AUTH-001", "testing", "--skip-temporal-validation"]);

    // @step Then the command exits with code 0
    assert_eq!(
        code2, 0,
        "expected exit 0 with --skip-temporal-validation; stdout={stdout2}, stderr={stderr2}"
    );
}

#[test]
fn scenario_cli_surfaces_blocking_hook_failure_on_stderr() {
    // @step Given a work unit "AUTH-001" exists with status "specifying"
    let ws = tempfile::tempdir().expect("tempdir");
    // The work unit satisfies review validation (rules + examples +
    // architecture notes + AST research attachment) so the pre-testing
    // blocking hook is actually reached before the transition is applied.
    write_work_units(
        ws.path(),
        &doc(
            "AUTH-001",
            "specifying",
            r#""virtualHooks": [{ "name": "must-pass", "event": "pre-testing", "command": "exit 1", "blocking": true }], "rules": [{ "id": 0, "text": "rule", "deleted": false }], "examples": [{ "id": 0, "text": "example", "deleted": false }], "architectureNotes": [{ "id": 0, "text": "note", "deleted": false }], "attachments": ["spec/attachments/AUTH-001/ast-research-login.json"]"#,
        ),
    );
    write_feature(
        ws.path(),
        "user-login",
        "AUTH-001",
        "Scenario: Login with valid credentials\n    Given I am on the login page\n    When I enter valid credentials\n    Then I should see the dashboard",
    );

    // @step And a blocking pre-transition hook is configured to fail
    // (configured above via the virtualHooks array)

    // @step When I run `fspec update-work-unit-status AUTH-001 testing`
    let (code, _stdout, stderr) =
        run_uwus(ws.path(), &["AUTH-001", "testing", "--skip-temporal-validation"]);

    // @step Then the command exits with a non-zero code
    assert_ne!(code, 0, "expected non-zero exit on blocking hook failure");

    // @step And the blocking hook stderr is wrapped in a system-reminder
    assert!(
        stderr.contains("<system-reminder>") || stderr.contains("BLOCKING HOOK"),
        "blocking hook stderr must be wrapped in a system-reminder; got:\n{stderr}"
    );
}

const TS_HELP_FIXTURE_UWUS: &str = include_str!("fixtures/help/update-work-unit-status.txt");

#[test]
fn scenario_cli_help_text_matches_byte_for_byte_fixture() {
    // @step When I run `fspec update-work-unit-status --help`
    let output = Command::new(fspec_bin())
        .arg("update-work-unit-status")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn update-work-unit-status --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout matches the help fixture exactly
    assert_eq!(stdout, TS_HELP_FIXTURE_UWUS);
}

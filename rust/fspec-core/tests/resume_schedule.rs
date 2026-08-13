// Feature: spec/features/resume-schedule-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `resume-schedule`
// (RPC-292). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.
//
// RED phase: `resume-schedule` is still a stub in `run_stub` returning
// FspecCoreError::NotYetPorted, so every assertion below FAILS until the
// port at `rust/fspec-core/src/commands/resume_schedule.rs` lands. These
// tests assert behavioural parity with the TypeScript implementation at
// `src/commands/schedule/pause-schedule.ts:51-71` (resumeSchedule).
//
// SUPERVISOR DECISION (orchestration-state.md): the missing-file divergence
// is APPROVED — a missing/empty spec/schedules.json yields the clean
// "Schedule '<name>' does not exist" error (NOT the TS TypeError crash).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "resume-schedule".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_schedules(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("schedules.json"), raw).expect("write schedules.json");
}

fn read_schedules_raw(project_root: &Path) -> String {
    fs::read_to_string(project_root.join("spec/schedules.json")).expect("read schedules.json")
}

fn read_schedules(project_root: &Path) -> Value {
    serde_json::from_str(&read_schedules_raw(project_root)).expect("schedules.json is valid JSON")
}

/// One shell schedule named `name` with the given status plus a full set of
/// canonical ScheduleEntry sibling fields, so preservation can be asserted.
fn one_shell_schedule(name: &str, status: &str) -> String {
    format!(
        r#"{{
  "version": "1.0.0",
  "schedules": {{
    "{name}": {{
      "name": "{name}",
      "cron": "0 2 * * *",
      "timezone": "UTC",
      "jobType": "shell",
      "overlapPolicy": "skip",
      "status": "{status}",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "2026-01-01T00:00:00.000Z",
      "command": "npm run build"
    }}
  }}
}}"#
    )
}

// ---------- scenarios ----------

#[test]
fn scenario_resume_paused_schedule_sets_status_active() {
    // Scenario: Resume a paused schedule sets its status to active

    // @step Given spec/schedules.json contains a shell schedule named 'nightly-review' with status 'paused'
    let tmp = TempDir::new().expect("tempdir");
    write_schedules(tmp.path(), &one_shell_schedule("nightly-review", "paused"));

    // @step When I dispatch the resume-schedule command with name='nightly-review' against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "name": "nightly-review" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/schedules.json now records the 'nightly-review' schedule with status 'active'
    let data = read_schedules(tmp.path());
    assert_eq!(
        data["schedules"]["nightly-review"]["status"].as_str(),
        Some("active"),
        "nightly-review must be active after resume-schedule; got {data}"
    );
}

#[test]
fn scenario_resume_missing_schedule_reports_does_not_exist() {
    // Scenario: Resuming a missing schedule reports it does not exist

    // @step Given spec/schedules.json contains a shell schedule named 'nightly-review' with status 'paused'
    let tmp = TempDir::new().expect("tempdir");
    let original = one_shell_schedule("nightly-review", "paused");
    write_schedules(tmp.path(), &original);

    // @step When I dispatch the resume-schedule command with name='ghost' against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "name": "ghost" })));

    // @step Then the dispatcher returns an error with message "Schedule 'ghost' does not exist"
    assert!(
        !result.success,
        "expected failure for missing schedule, got {result:?}"
    );
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Schedule 'ghost' does not exist"),
        "error must mention the missing schedule; got: {err}"
    );

    // @step And spec/schedules.json is unchanged
    assert_eq!(
        read_schedules_raw(tmp.path()),
        original,
        "schedules.json must be untouched on the missing-schedule error path"
    );
}

#[test]
fn scenario_resume_already_active_reports_already_active() {
    // Scenario: Resuming an already-active schedule reports it is already active

    // @step Given spec/schedules.json contains a shell schedule named 'nightly-review' with status 'active'
    let tmp = TempDir::new().expect("tempdir");
    let original = one_shell_schedule("nightly-review", "active");
    write_schedules(tmp.path(), &original);

    // @step When I dispatch the resume-schedule command with name='nightly-review' against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "name": "nightly-review" })));

    // @step Then the dispatcher returns an error with message "Schedule 'nightly-review' is already active"
    assert!(
        !result.success,
        "expected failure for already-active schedule, got {result:?}"
    );
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Schedule 'nightly-review' is already active"),
        "error must mention the schedule is already active; got: {err}"
    );

    // @step And spec/schedules.json is unchanged
    assert_eq!(
        read_schedules_raw(tmp.path()),
        original,
        "schedules.json must be untouched on the already-active error path"
    );
}

#[test]
fn scenario_resume_one_of_several_preserves_others_verbatim() {
    // Scenario: Resuming one of several schedules preserves the others verbatim

    // @step Given spec/schedules.json contains three schedules 'alpha' (paused), 'beta' (paused), and 'gamma' (active) with distinct cron, timezone, and jobType fields
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "version": "1.0.0",
  "schedules": {
    "alpha": {
      "name": "alpha",
      "cron": "0 1 * * *",
      "timezone": "UTC",
      "jobType": "shell",
      "overlapPolicy": "skip",
      "status": "paused",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "2026-01-01T00:00:00.000Z",
      "command": "echo alpha"
    },
    "beta": {
      "name": "beta",
      "cron": "0 9 * * 1-5",
      "timezone": "Australia/Sydney",
      "jobType": "agent",
      "overlapPolicy": "queue",
      "status": "paused",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "2026-01-02T00:00:00.000Z",
      "role": "standup-bot",
      "prompt": "Generate standup summary"
    },
    "gamma": {
      "name": "gamma",
      "cron": "0 3 * * *",
      "timezone": "Europe/London",
      "jobType": "shell",
      "overlapPolicy": "skip",
      "status": "active",
      "lastRunAt": "2026-01-05T03:00:00.000Z",
      "lastRunStatus": "completed",
      "createdAt": "2026-01-03T00:00:00.000Z",
      "command": "echo gamma"
    }
  }
}"#;
    write_schedules(tmp.path(), raw);
    let before = read_schedules(tmp.path());

    // @step When I dispatch the resume-schedule command with name='beta' against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "name": "beta" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let after = read_schedules(tmp.path());

    // @step And only the 'beta' schedule has status 'active'
    assert_eq!(
        after["schedules"]["beta"]["status"].as_str(),
        Some("active")
    );
    assert_eq!(
        after["schedules"]["alpha"]["status"].as_str(),
        Some("paused")
    );
    assert_eq!(
        after["schedules"]["gamma"]["status"].as_str(),
        Some("active")
    );

    // @step And the 'alpha' and 'gamma' schedules retain their original status and all sibling fields verbatim
    assert_eq!(
        after["schedules"]["alpha"], before["schedules"]["alpha"],
        "alpha must be byte-identical after resuming beta"
    );
    assert_eq!(
        after["schedules"]["gamma"], before["schedules"]["gamma"],
        "gamma must be byte-identical after resuming beta"
    );

    // @step And the 'beta' schedule retains its cron, timezone, and jobType fields verbatim
    assert_eq!(
        after["schedules"]["beta"]["cron"].as_str(),
        Some("0 9 * * 1-5")
    );
    assert_eq!(
        after["schedules"]["beta"]["timezone"].as_str(),
        Some("Australia/Sydney")
    );
    assert_eq!(
        after["schedules"]["beta"]["jobType"].as_str(),
        Some("agent")
    );
    assert_eq!(
        after["schedules"]["beta"]["role"].as_str(),
        Some("standup-bot")
    );
    assert_eq!(
        after["schedules"]["beta"]["prompt"].as_str(),
        Some("Generate standup summary")
    );
}

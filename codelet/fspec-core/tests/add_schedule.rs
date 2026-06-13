// Feature: spec/features/add-schedule-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-schedule`
// (RPC-191). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.
//
// RED phase: `add-schedule` is NOT yet in `PORTED_COMMANDS`; the stub at
// `codelet/fspec-core/src/commands/add_schedule.rs` returns
// `FspecCoreError::NotYetPorted`, so the dispatcher returns
// `success == false` with an error containing "not yet ported". Every
// success-path assertion below therefore FAILS until the Phase-C impl
// lands, and every error-path assertion checks for the SPECIFIC TS-parity
// validation message (which the NotYetPorted error does NOT contain) so it
// also fails loudly during RED.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-schedule".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_schedules(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("schedules.json"), raw).expect("write schedules.json");
}

/// Read and parse `spec/schedules.json` from disk; panics if missing/invalid.
fn read_schedules(project_root: &Path) -> Value {
    let path = project_root.join("spec").join("schedules.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("schedules.json must exist after add: {e}"));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("schedules.json not valid JSON: {e}\n{raw}"))
}

/// Return the named schedule entry if present in `schedules.json`, else None.
fn schedule_entry(project_root: &Path, name: &str) -> Option<Value> {
    let path = project_root.join("spec").join("schedules.json");
    let raw = fs::read_to_string(&path).ok()?;
    let data: Value = serde_json::from_str(&raw).ok()?;
    data.get("schedules")?.get(name).cloned()
}

// ---------- scenarios ----------

#[test]
fn scenario_add_agent_schedule_writes_entry_with_status_active() {
    // Scenario: Add an agent schedule writes the entry with status active

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the add-schedule command with name 'nightly-review' cron '0 2 * * *' timezone 'UTC' jobType 'agent' role 'Security reviewer' prompt 'Review src/'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "name": "nightly-review",
            "cron": "0 2 * * *",
            "timezone": "UTC",
            "jobType": "agent",
            "role": "Security reviewer",
            "prompt": "Review src/"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/schedules.json contains a schedule named 'nightly-review'
    let entry = schedule_entry(tmp.path(), "nightly-review")
        .expect("nightly-review entry must exist after add");

    // @step And the 'nightly-review' entry has jobType='agent', status='active', cron='0 2 * * *', timezone='UTC'
    assert_eq!(entry["jobType"].as_str(), Some("agent"));
    assert_eq!(entry["status"].as_str(), Some("active"));
    assert_eq!(entry["cron"].as_str(), Some("0 2 * * *"));
    assert_eq!(entry["timezone"].as_str(), Some("UTC"));

    // @step And the 'nightly-review' entry has role='Security reviewer' and prompt='Review src/'
    assert_eq!(entry["role"].as_str(), Some("Security reviewer"));
    assert_eq!(entry["prompt"].as_str(), Some("Review src/"));

    // @step And the 'nightly-review' entry has overlapPolicy='skip', lastRunAt=null, lastRunStatus=null
    assert_eq!(entry["overlapPolicy"].as_str(), Some("skip"));
    assert!(entry["lastRunAt"].is_null(), "lastRunAt must be null");
    assert!(entry["lastRunStatus"].is_null(), "lastRunStatus must be null");
}

#[test]
fn scenario_add_shell_schedule_writes_entry_with_default_skip_overlap_policy() {
    // Scenario: Add a shell schedule writes the entry with default skip overlap policy

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the add-schedule command with name 'daily-tests' cron '30 6 * * 1-5' timezone 'America/New_York' jobType 'shell' command 'npm test'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "name": "daily-tests",
            "cron": "30 6 * * 1-5",
            "timezone": "America/New_York",
            "jobType": "shell",
            "command": "npm test"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/schedules.json contains a schedule named 'daily-tests'
    let entry = schedule_entry(tmp.path(), "daily-tests")
        .expect("daily-tests entry must exist after add");

    // @step And the 'daily-tests' entry has jobType='shell', overlapPolicy='skip', command='npm test'
    assert_eq!(entry["jobType"].as_str(), Some("shell"));
    assert_eq!(entry["overlapPolicy"].as_str(), Some("skip"));
    assert_eq!(entry["command"].as_str(), Some("npm test"));
}

#[test]
fn scenario_schedules_json_is_auto_created_when_missing() {
    // Scenario: spec/schedules.json is auto-created when missing

    // @step Given a project root directory with no spec/schedules.json file
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/schedules.json").exists());

    // @step When I dispatch the add-schedule command with name 'weekly-deps' cron '0 9 * * 1' timezone 'Europe/London' jobType 'shell' command 'npx depcheck'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "name": "weekly-deps",
            "cron": "0 9 * * 1",
            "timezone": "Europe/London",
            "jobType": "shell",
            "command": "npx depcheck"
        }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/schedules.json exists with version '1.0.0'
    let data = read_schedules(tmp.path());
    assert_eq!(
        data["version"].as_str(),
        Some("1.0.0"),
        "auto-created schedules.json must carry version 1.0.0; got {data}"
    );

    // @step And spec/schedules.json contains a schedule named 'weekly-deps'
    assert!(
        schedule_entry(tmp.path(), "weekly-deps").is_some(),
        "weekly-deps entry must exist after add"
    );
}

#[test]
fn scenario_invalid_schedule_name_is_rejected_without_writing() {
    // Scenario: Invalid schedule name is rejected without writing

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the add-schedule command with name 'My Schedule' cron '0 2 * * *' timezone 'UTC' jobType 'shell' command 'echo hi'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "name": "My Schedule",
            "cron": "0 2 * * *",
            "timezone": "UTC",
            "jobType": "shell",
            "command": "echo hi"
        }),
    ));

    // @step Then the dispatcher returns an error mentioning lowercase hyphenated slugs
    assert!(!result.success, "expected failure, got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("lowercase, hyphenated slugs"),
        "error must mention lowercase hyphenated slugs; got: {err}"
    );

    // @step And spec/schedules.json contains no schedule named 'My Schedule'
    assert!(
        schedule_entry(tmp.path(), "My Schedule").is_none(),
        "no schedule must be written on validation failure"
    );
}

#[test]
fn scenario_cron_with_fewer_than_five_fields_is_rejected_without_writing() {
    // Scenario: Cron expression with fewer than five fields is rejected without writing

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the add-schedule command with name 'bad-cron' cron '0 2 * *' timezone 'UTC' jobType 'shell' command 'echo hi'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "name": "bad-cron",
            "cron": "0 2 * *",
            "timezone": "UTC",
            "jobType": "shell",
            "command": "echo hi"
        }),
    ));

    // @step Then the dispatcher returns an error mentioning expected 5 fields, got 4
    assert!(!result.success, "expected failure, got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("expected 5 fields") && err.contains("got 4"),
        "error must mention 'expected 5 fields' and 'got 4'; got: {err}"
    );

    // @step And spec/schedules.json contains no schedule named 'bad-cron'
    assert!(
        schedule_entry(tmp.path(), "bad-cron").is_none(),
        "no schedule must be written on validation failure"
    );
}

#[test]
fn scenario_invalid_timezone_is_rejected_without_writing() {
    // Scenario: Invalid timezone is rejected without writing

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the add-schedule command with name 'bad-tz' cron '0 2 * * *' timezone 'Not/AZone' jobType 'shell' command 'echo hi'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "name": "bad-tz",
            "cron": "0 2 * * *",
            "timezone": "Not/AZone",
            "jobType": "shell",
            "command": "echo hi"
        }),
    ));

    // @step Then the dispatcher returns an error mentioning the invalid timezone
    assert!(!result.success, "expected failure, got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Invalid timezone"),
        "error must mention the invalid timezone; got: {err}"
    );

    // @step And spec/schedules.json contains no schedule named 'bad-tz'
    assert!(
        schedule_entry(tmp.path(), "bad-tz").is_none(),
        "no schedule must be written on validation failure"
    );
}

#[test]
fn scenario_invalid_jobtype_is_rejected_without_writing() {
    // Scenario: Invalid jobType is rejected without writing

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the add-schedule command with name 'bad-type' cron '0 2 * * *' timezone 'UTC' jobType 'webhook' command 'echo hi'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "name": "bad-type",
            "cron": "0 2 * * *",
            "timezone": "UTC",
            "jobType": "webhook",
            "command": "echo hi"
        }),
    ));

    // @step Then the dispatcher returns an error mentioning jobType must be 'agent' or 'shell'
    assert!(!result.success, "expected failure, got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Invalid jobType") && err.contains("'agent' or 'shell'"),
        "error must mention jobType must be 'agent' or 'shell'; got: {err}"
    );

    // @step And spec/schedules.json contains no schedule named 'bad-type'
    assert!(
        schedule_entry(tmp.path(), "bad-type").is_none(),
        "no schedule must be written on validation failure"
    );
}

#[test]
fn scenario_agent_schedule_missing_role_and_prompt_is_rejected() {
    // Scenario: Agent schedule missing role and prompt is rejected

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the add-schedule command with name 'incomplete-agent' cron '0 2 * * *' timezone 'UTC' jobType 'agent' with no role or prompt
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "name": "incomplete-agent",
            "cron": "0 2 * * *",
            "timezone": "UTC",
            "jobType": "agent"
        }),
    ));

    // @step Then the dispatcher returns an error mentioning agent schedules require both role and prompt
    assert!(!result.success, "expected failure, got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Agent schedules require both role and prompt"),
        "error must mention agent schedules require both role and prompt; got: {err}"
    );

    // @step And spec/schedules.json contains no schedule named 'incomplete-agent'
    assert!(
        schedule_entry(tmp.path(), "incomplete-agent").is_none(),
        "no schedule must be written on validation failure"
    );
}

#[test]
fn scenario_shell_schedule_missing_command_is_rejected() {
    // Scenario: Shell schedule missing command is rejected

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch the add-schedule command with name 'incomplete-shell' cron '0 2 * * *' timezone 'UTC' jobType 'shell' with no command
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "name": "incomplete-shell",
            "cron": "0 2 * * *",
            "timezone": "UTC",
            "jobType": "shell"
        }),
    ));

    // @step Then the dispatcher returns an error mentioning shell schedules require a command
    assert!(!result.success, "expected failure, got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Shell schedules require a command"),
        "error must mention shell schedules require a command; got: {err}"
    );

    // @step And spec/schedules.json contains no schedule named 'incomplete-shell'
    assert!(
        schedule_entry(tmp.path(), "incomplete-shell").is_none(),
        "no schedule must be written on validation failure"
    );
}

#[test]
fn scenario_duplicate_schedule_name_is_rejected_and_existing_entry_preserved() {
    // Scenario: Duplicate schedule name is rejected and existing entry is preserved

    // @step Given spec/schedules.json already contains a schedule named 'nightly-review'
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "version": "1.0.0",
  "schedules": {
    "nightly-review": {
      "name": "nightly-review",
      "cron": "0 2 * * *",
      "timezone": "UTC",
      "jobType": "shell",
      "overlapPolicy": "skip",
      "status": "active",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "2026-01-01T00:00:00.000Z",
      "command": "echo original"
    }
  }
}"#;
    write_schedules(tmp.path(), raw);

    // @step When I dispatch the add-schedule command with name 'nightly-review' cron '0 3 * * *' timezone 'UTC' jobType 'shell' command 'echo dup'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "name": "nightly-review",
            "cron": "0 3 * * *",
            "timezone": "UTC",
            "jobType": "shell",
            "command": "echo dup"
        }),
    ));

    // @step Then the dispatcher returns an error mentioning the schedule already exists
    assert!(!result.success, "expected failure, got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("already exists"),
        "error must mention the schedule already exists; got: {err}"
    );

    // @step And the existing 'nightly-review' entry is unchanged
    let entry = schedule_entry(tmp.path(), "nightly-review")
        .expect("existing nightly-review entry must still be present");
    assert_eq!(
        entry["cron"].as_str(),
        Some("0 2 * * *"),
        "existing entry's cron must be unchanged (not overwritten by the duplicate)"
    );
    assert_eq!(
        entry["command"].as_str(),
        Some("echo original"),
        "existing entry's command must be unchanged"
    );
}

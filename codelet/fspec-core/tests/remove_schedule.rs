// Feature: spec/features/remove-schedule-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `remove-schedule`
// (RPC-280). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.
//
// RED phase: `remove-schedule` is NOT yet in `PORTED_COMMANDS`; the stub at
// `codelet/fspec-core/src/commands/remove_schedule.rs` returns
// `FspecCoreError::NotYetPorted`, so the dispatcher returns
// `success == false` with an error containing "not yet ported". Every
// success-path assertion below therefore FAILS until the Phase-C impl
// lands, and the error-path assertion checks for the SPECIFIC TS-parity
// "does not exist" message (which the NotYetPorted error does NOT contain).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "remove-schedule".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_schedules(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("schedules.json"), raw).expect("write schedules.json");
}

fn shell_entry(name: &str, cron: &str, command: &str, created: &str) -> String {
    format!(
        r#"    "{name}": {{
      "name": "{name}",
      "cron": "{cron}",
      "timezone": "UTC",
      "jobType": "shell",
      "overlapPolicy": "skip",
      "status": "active",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "{created}",
      "command": "{command}"
    }}"#
    )
}

/// Return the named schedule entry if present, else None.
fn schedule_entry(project_root: &Path, name: &str) -> Option<Value> {
    let path = project_root.join("spec").join("schedules.json");
    let raw = fs::read_to_string(&path).ok()?;
    let data: Value = serde_json::from_str(&raw).ok()?;
    data.get("schedules")?.get(name).cloned()
}

/// Return the ordered list of schedule keys from disk.
fn schedule_keys(project_root: &Path) -> Vec<String> {
    let path = project_root.join("spec").join("schedules.json");
    let raw = fs::read_to_string(&path).expect("schedules.json must exist");
    let data: Value = serde_json::from_str(&raw).expect("schedules.json valid JSON");
    data["schedules"]
        .as_object()
        .expect("schedules object")
        .keys()
        .cloned()
        .collect()
}

// ---------- scenarios ----------

#[test]
fn scenario_remove_existing_schedule_deletes_only_that_entry() {
    // Scenario: Remove an existing schedule deletes only that entry

    // @step Given spec/schedules.json contains schedules named 'nightly-review' and 'daily-tests'
    let tmp = TempDir::new().expect("tempdir");
    let raw = format!(
        "{{\n  \"version\": \"1.0.0\",\n  \"schedules\": {{\n{},\n{}\n  }}\n}}",
        shell_entry(
            "nightly-review",
            "0 2 * * *",
            "echo n",
            "2026-01-01T00:00:00.000Z"
        ),
        shell_entry(
            "daily-tests",
            "30 6 * * 1-5",
            "echo d",
            "2026-01-02T00:00:00.000Z"
        ),
    );
    write_schedules(tmp.path(), &raw);

    // @step When I dispatch the remove-schedule command with name 'nightly-review'
    let result = dispatch_command(req(tmp.path(), json!({ "name": "nightly-review" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And spec/schedules.json contains no schedule named 'nightly-review'
    assert!(
        schedule_entry(tmp.path(), "nightly-review").is_none(),
        "nightly-review must be deleted"
    );

    // @step And spec/schedules.json still contains a schedule named 'daily-tests'
    assert!(
        schedule_entry(tmp.path(), "daily-tests").is_some(),
        "daily-tests must be preserved"
    );
}

#[test]
fn scenario_removing_nonexistent_schedule_errors_and_leaves_file_unchanged() {
    // Scenario: Removing a non-existent schedule returns an error and leaves the file unchanged

    // @step Given spec/schedules.json contains a schedule named 'daily-tests'
    let tmp = TempDir::new().expect("tempdir");
    let raw = format!(
        "{{\n  \"version\": \"1.0.0\",\n  \"schedules\": {{\n{}\n  }}\n}}",
        shell_entry(
            "daily-tests",
            "30 6 * * 1-5",
            "echo d",
            "2026-01-02T00:00:00.000Z"
        ),
    );
    write_schedules(tmp.path(), &raw);

    // @step When I dispatch the remove-schedule command with name 'does-not-exist'
    let result = dispatch_command(req(tmp.path(), json!({ "name": "does-not-exist" })));

    // @step Then the dispatcher returns an error mentioning the schedule does not exist
    assert!(!result.success, "expected failure, got {result:?}");
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("does not exist"),
        "error must mention the schedule does not exist; got: {err}"
    );

    // @step And spec/schedules.json still contains a schedule named 'daily-tests'
    assert!(
        schedule_entry(tmp.path(), "daily-tests").is_some(),
        "daily-tests must be left untouched on failure"
    );
}

#[test]
fn scenario_removing_schedule_preserves_insertion_order_of_remaining() {
    // Scenario: Removing a schedule preserves the insertion order of the remaining schedules

    // @step Given spec/schedules.json contains schedules declared in order ZED, AAA, MID
    let tmp = TempDir::new().expect("tempdir");
    let raw = format!(
        "{{\n  \"version\": \"1.0.0\",\n  \"schedules\": {{\n{},\n{},\n{}\n  }}\n}}",
        shell_entry("ZED", "0 1 * * *", "echo z", "2026-01-01T00:00:00.000Z"),
        shell_entry("AAA", "0 2 * * *", "echo a", "2026-01-02T00:00:00.000Z"),
        shell_entry("MID", "0 3 * * *", "echo m", "2026-01-03T00:00:00.000Z"),
    );
    write_schedules(tmp.path(), &raw);

    // @step When I dispatch the remove-schedule command with name 'AAA'
    let result = dispatch_command(req(tmp.path(), json!({ "name": "AAA" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the remaining schedules are in order ZED, MID
    let keys = schedule_keys(tmp.path());
    assert_eq!(
        keys,
        vec!["ZED".to_string(), "MID".to_string()],
        "remaining schedules must preserve insertion order ZED, MID; got {keys:?}"
    );
}

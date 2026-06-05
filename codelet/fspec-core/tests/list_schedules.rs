// Feature: spec/features/list-schedules-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `list-schedules`
// (RPC-250). Each scenario maps to exactly one #[test] function with
// @step comments mirroring the Gherkin steps verbatim.
//
// GREEN phase: `list-schedules` is registered in `PORTED_COMMANDS` and
// the impl at `codelet/fspec-core/src/commands/list_schedules.rs`
// produces the canonical `{schedules, columns}` payload (JSON) or the
// documented tab-separated text rendering. The tests below assert
// behavioural parity with the TypeScript implementation at
// `src/commands/schedule/list-schedules.ts`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "list-schedules".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_schedules(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("schedules.json"), raw).expect("write schedules.json");
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

fn expected_columns() -> Vec<&'static str> {
    vec![
        "name", "cron", "timezone", "type", "status", "lastRun", "nextRun",
    ]
}

fn assert_columns_match(data: &Value, raw: &str) {
    let cols = data["columns"].as_array().expect("columns array");
    let actual: Vec<&str> = cols.iter().map(|v| v.as_str().unwrap_or("")).collect();
    assert_eq!(
        actual,
        expected_columns(),
        "columns mismatch; got {raw}"
    );
}

// ---------- scenarios ----------

#[test]
fn scenario_returns_empty_schedules_with_columns_when_file_missing() {
    // Scenario: Returns empty schedules with canonical columns when spec/schedules.json does not exist

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch the list-schedules command against that project root with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the parsed JSON has schedules array of length 0
    let data = parse_data(&result.data);
    assert_eq!(
        data["schedules"].as_array().map(Vec::len),
        Some(0),
        "expected empty schedules array, got {}",
        result.data
    );

    // @step Then the parsed JSON has columns equal to ["name","cron","timezone","type","status","lastRun","nextRun"]
    assert_columns_match(&data, &result.data);

    // @step Then spec/schedules.json does not exist after the call
    assert!(
        !tmp.path().join("spec/schedules.json").exists(),
        "list-schedules must NOT auto-create spec/schedules.json"
    );
}

#[test]
fn scenario_returns_schedule_entries_verbatim_when_populated() {
    // Scenario: Returns schedule entries verbatim when spec/schedules.json is populated

    // @step Given spec/schedules.json contains a shell schedule named 'nightly-build' with cron '0 2 * * *' and an agent schedule named 'morning-standup' with cron '0 9 * * 1-5' in that order
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "version": "1.0.0",
  "schedules": {
    "nightly-build": {
      "name": "nightly-build",
      "cron": "0 2 * * *",
      "timezone": "UTC",
      "jobType": "shell",
      "overlapPolicy": "skip",
      "status": "active",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "2026-01-01T00:00:00.000Z",
      "command": "npm run build"
    },
    "morning-standup": {
      "name": "morning-standup",
      "cron": "0 9 * * 1-5",
      "timezone": "Australia/Sydney",
      "jobType": "agent",
      "overlapPolicy": "skip",
      "status": "active",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "2026-01-02T00:00:00.000Z",
      "role": "standup-bot",
      "prompt": "Generate standup summary"
    }
  }
}"#;
    write_schedules(tmp.path(), raw);

    // @step When I dispatch list-schedules with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let data = parse_data(&result.data);
    let arr = data["schedules"].as_array().expect("schedules array");

    // @step Then the schedules array contains exactly two entries
    assert_eq!(arr.len(), 2, "expected 2 schedules, got {arr:?}");

    // @step Then the first schedule has name='nightly-build', cron='0 2 * * *', jobType='shell', status='active'
    assert_eq!(arr[0]["name"].as_str(), Some("nightly-build"));
    assert_eq!(arr[0]["cron"].as_str(), Some("0 2 * * *"));
    assert_eq!(arr[0]["jobType"].as_str(), Some("shell"));
    assert_eq!(arr[0]["status"].as_str(), Some("active"));

    // @step Then the second schedule has name='morning-standup', cron='0 9 * * 1-5', jobType='agent'
    assert_eq!(arr[1]["name"].as_str(), Some("morning-standup"));
    assert_eq!(arr[1]["cron"].as_str(), Some("0 9 * * 1-5"));
    assert_eq!(arr[1]["jobType"].as_str(), Some("agent"));

    // @step Then the parsed JSON has columns equal to ["name","cron","timezone","type","status","lastRun","nextRun"]
    assert_columns_match(&data, &result.data);
}

#[test]
fn scenario_empty_schedules_map_is_no_entries_but_columns_present() {
    // Scenario: Treats empty schedules map as no schedules but still emits columns

    // @step Given spec/schedules.json exists and parses to an object whose 'schedules' field is the empty object
    let tmp = TempDir::new().expect("tempdir");
    write_schedules(tmp.path(), r#"{ "version": "1.0.0", "schedules": {} }"#);

    // @step When I dispatch list-schedules with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    let data = parse_data(&result.data);

    // @step Then the schedules array has length 0
    assert_eq!(
        data["schedules"].as_array().map(Vec::len),
        Some(0),
        "expected empty schedules array, got {}",
        result.data
    );

    // @step Then the parsed JSON has columns equal to ["name","cron","timezone","type","status","lastRun","nextRun"]
    assert_columns_match(&data, &result.data);
}

#[test]
fn scenario_swallows_invalid_json_as_empty_with_columns() {
    // Scenario: Swallows invalid JSON as empty result with canonical columns

    // @step Given spec/schedules.json exists but contains the malformed bytes '{ not json'
    let tmp = TempDir::new().expect("tempdir");
    write_schedules(tmp.path(), "{ not json");

    // @step When I dispatch list-schedules with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "TS readJSON-with-default swallows parse errors; expected success=true, got {result:?}"
    );

    let data = parse_data(&result.data);

    // @step Then the schedules array has length 0
    assert_eq!(
        data["schedules"].as_array().map(Vec::len),
        Some(0),
        "expected empty schedules array on parse failure, got {}",
        result.data
    );

    // @step Then the parsed JSON has columns equal to ["name","cron","timezone","type","status","lastRun","nextRun"]
    assert_columns_match(&data, &result.data);

    // @step Then spec/schedules.json still contains the original malformed bytes after the call
    let bytes = fs::read_to_string(tmp.path().join("spec/schedules.json"))
        .expect("schedules.json must still exist");
    assert_eq!(
        bytes, "{ not json",
        "list-schedules must NOT overwrite malformed schedules.json; got {bytes:?}"
    );
}

#[test]
fn scenario_preserves_insertion_order_of_schedules() {
    // Scenario: Preserves insertion order of schedules (not alphabetical)

    // @step Given spec/schedules.json contains three schedule entries declared in order ZED, AAA, MID
    let tmp = TempDir::new().expect("tempdir");
    // Hand-write so object key order is preserved on the wire.
    let raw = r#"{
  "version": "1.0.0",
  "schedules": {
    "ZED": {
      "name": "ZED",
      "cron": "0 1 * * *",
      "timezone": "UTC",
      "jobType": "shell",
      "overlapPolicy": "skip",
      "status": "active",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "2026-01-01T00:00:00.000Z",
      "command": "echo z"
    },
    "AAA": {
      "name": "AAA",
      "cron": "0 2 * * *",
      "timezone": "UTC",
      "jobType": "shell",
      "overlapPolicy": "skip",
      "status": "active",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "2026-01-02T00:00:00.000Z",
      "command": "echo a"
    },
    "MID": {
      "name": "MID",
      "cron": "0 3 * * *",
      "timezone": "UTC",
      "jobType": "shell",
      "overlapPolicy": "skip",
      "status": "active",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "2026-01-03T00:00:00.000Z",
      "command": "echo m"
    }
  }
}"#;
    write_schedules(tmp.path(), raw);

    // @step When I dispatch list-schedules with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    let data = parse_data(&result.data);
    let arr = data["schedules"].as_array().expect("schedules array");

    // @step Then the schedules array contains three entries in order ZED, AAA, MID
    assert_eq!(arr.len(), 3, "expected 3 schedules, got {arr:?}");
    assert_eq!(arr[0]["name"].as_str(), Some("ZED"));
    assert_eq!(arr[1]["name"].as_str(), Some("AAA"));
    assert_eq!(arr[2]["name"].as_str(), Some("MID"));
}

#[test]
fn scenario_json_format_two_space_indent_for_empty_case() {
    // Scenario: JSON format emits two-space indented payload for the empty/missing case

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch list-schedules with format='json'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "json" })));
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data starts with the exact string "{\n  \"schedules\": [],\n"
    assert!(
        result.data.starts_with("{\n  \"schedules\": [],\n"),
        "expected 2-space indented JSON opener; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact substring "\"columns\": ["
    assert!(
        result.data.contains("\"columns\": ["),
        "missing 'columns: [' substring; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact substring "\"name\""
    assert!(
        result.data.contains("\"name\""),
        "missing '\"name\"' substring; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_text_format_empty_prints_no_schedules_sentinel() {
    // Scenario: Text format prints 'No schedules configured.' sentinel for the empty/missing case

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch list-schedules with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the exact line 'No schedules configured.'
    assert!(
        result.data.lines().any(|l| l == "No schedules configured."),
        "missing exact line 'No schedules configured.'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line 'Use `fspec add-schedule` to create a schedule.'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "Use `fspec add-schedule` to create a schedule."),
        "missing exact line 'Use `fspec add-schedule` to create a schedule.'; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_text_format_populated_help_example_layout() {
    // Scenario: Text format renders the populated case using the documented help-example layout

    // @step Given spec/schedules.json contains one shell schedule named 'nightly-build' with cron '0 2 * * *' timezone 'UTC' status 'active' and lastRunAt null
    let tmp = TempDir::new().expect("tempdir");
    let raw = r#"{
  "version": "1.0.0",
  "schedules": {
    "nightly-build": {
      "name": "nightly-build",
      "cron": "0 2 * * *",
      "timezone": "UTC",
      "jobType": "shell",
      "overlapPolicy": "skip",
      "status": "active",
      "lastRunAt": null,
      "lastRunStatus": null,
      "createdAt": "2026-01-01T00:00:00.000Z",
      "command": "npm run build"
    }
  }
}"#;
    write_schedules(tmp.path(), raw);

    // @step When I dispatch list-schedules with format='text'
    let result = dispatch_command(req(tmp.path(), json!({ "format": "text" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the tab-separated header line 'Name\tCron\tTimezone\tType\tStatus\tLast Run\tNext Run'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "Name\tCron\tTimezone\tType\tStatus\tLast Run\tNext Run"),
        "missing tab-separated header line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains a line that begins with 'nightly-build\t0 2 * * *\tUTC\tshell\tactive\t'
    assert!(
        result
            .data
            .lines()
            .any(|l| l.starts_with("nightly-build\t0 2 * * *\tUTC\tshell\tactive\t")),
        "missing row line starting with 'nightly-build\\t...'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line 'Total: 1 schedule(s)'
    assert!(
        result.data.lines().any(|l| l == "Total: 1 schedule(s)"),
        "missing 'Total: 1 schedule(s)' summary line; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_default_format_is_text() {
    // Scenario: Default format (no format key supplied) is text

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch list-schedules with an empty args object {}
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "{result:?}");

    // @step Then the DispatchResult.data contains the exact line 'No schedules configured.'
    assert!(
        result.data.lines().any(|l| l == "No schedules configured."),
        "default format must be text and render the empty sentinel; got: {:?}",
        result.data
    );
}

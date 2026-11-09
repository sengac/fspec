#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/tool-detection-check-functions-not-integrated-into-workflow.feature
//
// Moving a work unit to validating prints the configure-tools test command
// check and quality commands check (CONFIG-003; COV-071 moved CONFIG-002's
// chained-command and date-aware-query scenarios here). Each scenario maps to
// one #[test] fn with @step comments mirroring the Gherkin steps verbatim.
// AUTH-001 is a task so the move to validating needs no feature coverage.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

fn dispatch(project_root: &Path, command: &str, args: Value) -> codelet_fspec_core::DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

fn write_config(project_root: &Path, config: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("fspec-config.json"),
        serde_json::to_string_pretty(config).expect("config JSON"),
    )
    .expect("write fspec-config.json");
}

/// A board holding the task AUTH-001 in implementing.
fn write_implementing_task(project_root: &Path) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let board = json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": {
            "AUTH-001": {
                "id": "AUTH-001", "title": "Login", "type": "task", "status": "implementing",
                "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": [], "specifying": [], "testing": [], "implementing": ["AUTH-001"],
            "validating": [], "done": [], "blocked": []
        }
    });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&board).expect("board JSON"),
    )
    .expect("write work-units.json");
}

/// Move AUTH-001 to validating; returns the command's stdout.
fn move_to_validating(project_root: &Path) -> String {
    let result = dispatch(
        project_root,
        "update-work-unit-status",
        json!({"workUnitId": "AUTH-001", "status": "validating", "skipTemporalValidation": true}),
    );
    assert!(result.success, "move to validating failed: {result:?}");
    let raw = fs::read_to_string(project_root.join("spec/work-units.json")).expect("board");
    let board: Value = serde_json::from_str(&raw).expect("board JSON");
    assert_eq!(board["workUnits"]["AUTH-001"]["status"], "validating");
    result.data
}

#[test]
fn emit_system_reminder_when_moving_to_validating_with_no_config() {
    // @step Given spec/fspec-config.json sets agent 'claude' but has no tools.test.command configured
    let tmp = TempDir::new().expect("tempdir");
    write_config(tmp.path(), &json!({"agent": "claude"}));
    // @step And work unit AUTH-001 is in implementing
    write_implementing_task(tmp.path());

    // @step When work unit AUTH-001 moves to validating through update-work-unit-status
    // @step Then the status change succeeds
    let out = move_to_validating(tmp.path());

    // @step And system-reminder should be emitted to console output
    // @step And system-reminder should contain text: 'NO TEST COMMAND CONFIGURED'
    assert!(
        out.contains("<system-reminder>\nNO TEST COMMAND CONFIGURED"),
        "{out}"
    );
    // @step And system-reminder should tell AI to run: fspec configure-tools --test-command <cmd>
    assert!(
        out.contains("fspec configure-tools --test-command <cmd>"),
        "{out}"
    );
}

#[test]
fn emit_system_reminder_when_moving_to_validating_with_config_present() {
    // @step Given spec/fspec-config.json sets agent 'claude' and has tools.test.command = 'npm test'
    let tmp = TempDir::new().expect("tempdir");
    write_config(
        tmp.path(),
        &json!({"agent": "claude", "tools": {"test": {"command": "npm test"}}}),
    );
    // @step And work unit AUTH-001 is in implementing
    write_implementing_task(tmp.path());

    // @step When work unit AUTH-001 moves to validating through update-work-unit-status
    // @step Then the status change succeeds
    let out = move_to_validating(tmp.path());

    // @step And system-reminder should be emitted to console output
    // @step And system-reminder should contain text: 'RUN TESTS'
    assert!(out.contains("<system-reminder>\nRUN TESTS"), "{out}");
    // @step And system-reminder should contain text: 'Run tests: npm test'
    assert!(out.contains("Run tests: npm test\n"), "{out}");
    // @step And the output should not prompt the AI to configure tools
    assert!(!out.contains("NO TEST COMMAND CONFIGURED"), "{out}");
    assert!(
        !out.contains("fspec configure-tools --test-command"),
        "{out}"
    );
}

#[test]
fn emit_quality_check_reminder_when_moving_to_validating() {
    // @step Given spec/fspec-config.json sets agent 'claude' and has tools.qualityCheck.commands = ['eslint .', 'prettier --check .']
    let tmp = TempDir::new().expect("tempdir");
    write_config(
        tmp.path(),
        &json!({
            "agent": "claude",
            "tools": {"qualityCheck": {"commands": ["eslint .", "prettier --check ."]}}
        }),
    );
    // @step And work unit AUTH-001 is in implementing
    write_implementing_task(tmp.path());

    // @step When work unit AUTH-001 moves to validating through update-work-unit-status
    // @step Then the status change succeeds
    let out = move_to_validating(tmp.path());

    // @step And system-reminder should be emitted to console output
    // @step And system-reminder should contain text: 'RUN QUALITY CHECKS'
    assert!(
        out.contains("<system-reminder>\nRUN QUALITY CHECKS"),
        "{out}"
    );
    // @step And system-reminder should contain chained command: 'eslint . && prettier --check .'
    assert!(
        out.contains("Run quality checks: eslint . && prettier --check .\n"),
        "{out}"
    );
}

#[test]
fn chain_multiple_test_frameworks_when_ai_provides_chained_command() {
    // @step Given AI ran configure-tools with the test command 'cargo test && npm test'
    let tmp = TempDir::new().expect("tempdir");
    let configured = dispatch(
        tmp.path(),
        "configure-tools",
        json!({"testCommand": "cargo test && npm test"}),
    );
    assert!(configured.success, "configure-tools failed: {configured:?}");
    // @step And work unit AUTH-001 is in implementing
    write_implementing_task(tmp.path());

    // @step When work unit AUTH-001 moves to validating through update-work-unit-status
    let out = move_to_validating(tmp.path());

    // @step Then spec/fspec-config.json should store tools.test.command = 'cargo test && npm test'
    let raw = fs::read_to_string(tmp.path().join("spec/fspec-config.json")).expect("config");
    let config: Value = serde_json::from_str(&raw).expect("config JSON");
    assert_eq!(config["tools"]["test"]["command"], "cargo test && npm test");
    // @step And the output should contain: 'Run tests: cargo test && npm test'
    assert!(out.contains("Run tests: cargo test && npm test\n"), "{out}");
}

#[test]
fn include_date_aware_search_queries_when_no_tools_detected() {
    // @step Given spec/fspec-config.json does not exist
    let tmp = TempDir::new().expect("tempdir");
    // @step And work unit AUTH-001 is in implementing
    write_implementing_task(tmp.path());
    assert!(!tmp.path().join("spec/fspec-config.json").exists());

    // @step When work unit AUTH-001 moves to validating through update-work-unit-status
    let out = move_to_validating(tmp.path());

    // @step Then the reminder should include the search query 'best <platform> testing tools' followed by a four-digit year
    let marker = "Query: \"best <platform> testing tools ";
    let start = out.find(marker).map(|i| i + marker.len());
    let year = start.and_then(|s| out.get(s..s + 5));
    assert!(
        year.is_some_and(|y| y[..4].chars().all(|c| c.is_ascii_digit()) && y.ends_with('"')),
        "{out}"
    );
    // @step And the reminder should tell the AI to replace <platform> with the detected project type
    assert!(
        out.contains(
            "Replace <platform> with detected project type (Node.js, Python, Rust, Go, etc.)"
        ),
        "{out}"
    );
}

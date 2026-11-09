//! BUG-538 — guidance points at help commands that exist.
//!
//! `fspec help <group>` was never ported to the Rust CLI. The specifying
//! reminder, the TOML slash command written by init and the bootstrap
//! Complete Command Reference step pointed at it; they now point at
//! `fspec bootstrap`, `fspec --help` and `fspec <command> --help`.
//!
//! Feature: spec/features/guidance-points-at-existing-help-commands.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::json;

const HELP_GROUPS: [&str; 6] = ["specs", "work", "discovery", "metrics", "setup", "hooks"];

fn dispatch(project_root: &Path, command: &str, args: serde_json::Value) -> DispatchResult {
    let result = dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    });
    assert!(result.success, "{command} failed: {result:?}");
    result
}

fn assert_no_help_group(text: &str) {
    for group in HELP_GROUPS {
        let mention = format!("fspec help {group}");
        assert!(!text.contains(&mention), "still names `{mention}`:\n{text}");
    }
}

#[test]
fn the_specifying_reminder_points_at_the_bootstrap_example_mapping_section() {
    // @step Given a work unit in backlog
    let root = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("spec")).expect("mkdir spec");
    let board = json!({
        "version": "0.7.1",
        "meta": {"version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z"},
        "workUnits": {"AUTH-001": {"id": "AUTH-001", "title": "Login", "type": "story",
            "status": "backlog", "createdAt": "2026-06-01T00:00:00.000Z",
            "updatedAt": "2026-06-01T00:00:00.000Z"}},
        "states": {"backlog": ["AUTH-001"], "specifying": [], "testing": [], "implementing": [],
            "validating": [], "done": [], "blocked": []}
    });
    fs::write(
        root.path().join("spec/work-units.json"),
        serde_json::to_string_pretty(&board).expect("serialize board"),
    )
    .expect("write work-units.json");

    // @step When the agent moves it to specifying
    let result = dispatch(
        root.path(),
        "update-work-unit-status",
        json!({"workUnitId": "AUTH-001", "status": "specifying"}),
    );
    let text = format!(
        "{}\n{}",
        result.data,
        result.system_reminder.unwrap_or_default()
    );

    // @step Then the reminder ends with "For more: fspec bootstrap (Example Mapping section)"
    let pointer =
        "For more: fspec bootstrap (Example Mapping section)\n\nDO NOT write tests or code yet.";
    assert!(text.contains(pointer), "pointer missing:\n{text}");

    // @step And the reminder does not contain "fspec help discovery"
    assert!(!text.contains("fspec help discovery"), "{text}");
}

#[test]
fn the_toml_slash_command_lists_fspec_sync_version_and_fspec_bootstrap() {
    // @step Given an empty project
    let root = tempfile::tempdir().expect("tempdir");

    // @step When fspec init installs the Gemini CLI agent
    dispatch(root.path(), "init", json!({"agent": ["gemini"]}));
    let toml = fs::read_to_string(root.path().join(".gemini/commands/fspec.toml"))
        .expect("read fspec.toml");

    // @step Then .gemini/commands/fspec.toml tells the agent to run fspec --sync-version with the fspec version and then fspec bootstrap
    let steps = format!(
        "1. fspec --sync-version {}\n2. fspec bootstrap\n",
        env!("CARGO_PKG_VERSION")
    );
    assert!(toml.contains(&steps), "expected the two steps:\n{toml}");

    // @step And it names no fspec help group command
    assert_no_help_group(&toml);
}

#[test]
fn the_bootstrap_command_reference_points_at_per_command_help() {
    // @step Given any project
    let root = tempfile::tempdir().expect("tempdir");

    // @step When the agent runs fspec bootstrap
    let output = dispatch(root.path(), "bootstrap", json!({})).data;

    // @step Then the Complete Command Reference step points at fspec --help and fspec <command> --help
    let start = output
        .find("## Step 12: Complete Command Reference")
        .expect("Complete Command Reference step present");
    let step = &output[start..];
    let step = &step[..step.find("\n---").expect("step ends with a rule")];
    assert!(
        step.contains("`fspec --help`") && step.contains("`fspec <command> --help`"),
        "per-command help pointers missing:\n{step}"
    );

    // @step And the output names none of fspec help specs, work, discovery, metrics, setup or hooks
    assert_no_help_group(&output);
}

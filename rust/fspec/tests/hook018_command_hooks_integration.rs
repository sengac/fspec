#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/integrate-hooks-into-all-commands.feature
//
// HOOK-018: every fspec command runs its pre-<command> and post-<command>
// hooks from spec/fspec-hooks.json, through both front doors: the fspec CLI
// (the real binary) and agent tool calls (dispatch_command).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

/// A project whose board holds AUTH-001 and DASH-001 in `specifying`, with
/// the given tags, and whose spec/fspec-hooks.json is `hooks`. They are
/// tasks, so moving them to implementing passes every gate.
fn project(hooks: Value, tags: &[(&str, Value)]) -> TempDir {
    project_of("task", hooks, tags)
}

/// As [`project`], with work units of type `kind` (a story can be moved to
/// testing, which the blocking pre-hook scenarios try).
fn project_of(kind: &str, hooks: Value, tags: &[(&str, Value)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let spec = dir.path().join("spec");
    fs::create_dir_all(&spec).unwrap();
    let mut units = serde_json::Map::new();
    for id in ["AUTH-001", "DASH-001"] {
        let mut unit = json!({
            "id": id, "title": format!("{id} unit"), "type": kind, "status": "specifying",
            "createdAt": "2026-11-07T00:00:00.000Z", "updatedAt": "2026-11-07T00:00:00.000Z",
            "stateHistory": [
                {"state": "backlog", "timestamp": "2026-11-07T00:00:00.000Z"},
                {"state": "specifying", "timestamp": "2026-11-07T00:00:01.000Z"}
            ]
        });
        if let Some((_, value)) = tags.iter().find(|(tagged, _)| *tagged == id) {
            unit["tags"] = value.clone();
        }
        units.insert(id.to_string(), unit);
    }
    let board = json!({
        "meta": {"version": "1.0.0", "lastUpdated": "2026-11-07T00:00:00.000Z"},
        "workUnits": units,
        "states": {"backlog": [], "specifying": ["AUTH-001", "DASH-001"], "testing": [],
                   "implementing": [], "validating": [], "done": [], "blocked": []}
    });
    fs::write(spec.join("work-units.json"), board.to_string()).unwrap();
    fs::write(
        spec.join("fspec-hooks.json"),
        json!({"hooks": hooks}).to_string(),
    )
    .unwrap();
    dir
}

fn script(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn fspec(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fspec"))
        .args(args)
        .current_dir(root)
        .env("HOME", root)
        .env("XDG_CONFIG_HOME", root.join(".config"))
        .env("FSPEC_DATA_DIR", root.join(".fspec-data"))
        .output()
        .unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn status_of(root: &Path, id: &str) -> String {
    let raw = fs::read_to_string(root.join("spec/work-units.json")).unwrap();
    let board: Value = serde_json::from_str(&raw).unwrap();
    board["workUnits"][id]["status"]
        .as_str()
        .unwrap()
        .to_string()
}

fn status_in(saved_board: &Path, id: &str) -> String {
    let board: Value = serde_json::from_str(&fs::read_to_string(saved_board).unwrap()).unwrap();
    board["workUnits"][id]["status"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn command_triggers_pre_and_post_hooks() {
    // @step Given I have hooks configured for "pre-update-work-unit-status" and "post-update-work-unit-status"
    let dir = project(
        json!({
            "pre-update-work-unit-status": [{"name": "before", "command": "spec/hooks/before.sh"}],
            "post-update-work-unit-status": [{"name": "after", "command": "spec/hooks/after.sh"}]
        }),
        &[],
    );
    let root = dir.path();
    script(
        root,
        "spec/hooks/before.sh",
        "cat > pre-context.json\ncp spec/work-units.json pre-board.json",
    );
    script(
        root,
        "spec/hooks/after.sh",
        "cat > post-context.json\ncp spec/work-units.json post-board.json",
    );
    // @step When I run "fspec update-work-unit-status AUTH-001 implementing"
    let output = fspec(
        root,
        &["update-work-unit-status", "AUTH-001", "implementing"],
    );
    assert!(output.status.success(), "{}", text(&output));
    // @step Then the pre-update-work-unit-status hooks should execute before status change
    assert_eq!(
        status_in(&root.join("pre-board.json"), "AUTH-001"),
        "specifying"
    );
    // @step And the post-update-work-unit-status hooks should execute after status change
    assert_eq!(
        status_in(&root.join("post-board.json"), "AUTH-001"),
        "implementing"
    );
    let pre: Value =
        serde_json::from_str(&fs::read_to_string(root.join("pre-context.json")).unwrap()).unwrap();
    let post: Value =
        serde_json::from_str(&fs::read_to_string(root.join("post-context.json")).unwrap()).unwrap();
    // @step And the hooks should receive context with workUnitId "AUTH-001"
    assert_eq!(pre["workUnitId"], "AUTH-001");
    assert_eq!(post["workUnitId"], "AUTH-001");
    // @step And the hooks should receive context with event name
    assert_eq!(pre["event"], "pre-update-work-unit-status");
    assert_eq!(post["event"], "post-update-work-unit-status");
    // @step And the hooks should receive context with timestamp
    assert!(
        pre["timestamp"].as_str().is_some_and(|t| !t.is_empty()),
        "{pre}"
    );
    assert!(
        post["timestamp"].as_str().is_some_and(|t| !t.is_empty()),
        "{post}"
    );
}

#[test]
fn pre_hook_validates_preconditions_and_blocks_command() {
    // @step Given I have a blocking pre-hook "validate-feature-file" for "pre-update-work-unit-status"
    let dir = project_of(
        "story",
        json!({"pre-update-work-unit-status": [
            {"name": "validate-feature-file", "command": "spec/hooks/validate-feature-file.sh", "blocking": true}
        ]}),
        &[],
    );
    let root = dir.path();
    // @step And the hook checks that work unit has a linked feature file
    script(
        root,
        "spec/hooks/validate-feature-file.sh",
        "id=$(sed -n 's/.*\"workUnitId\":\"\\([^\"]*\\)\".*/\\1/p')\n\
         grep -l \"@$id\" spec/features/*.feature >/dev/null 2>&1 && exit 0\n\
         echo \"No feature file is linked to $id\" >&2\nexit 1",
    );
    // @step And work unit "AUTH-001" has no linked feature file
    assert!(!root.join("spec/features").exists());
    // @step When I run "fspec update-work-unit-status AUTH-001 testing"
    let output = fspec(root, &["update-work-unit-status", "AUTH-001", "testing"]);
    let all = text(&output);
    // @step Then the pre-hook should execute and fail
    assert!(
        all.contains("No feature file is linked to AUTH-001"),
        "{all}"
    );
    // @step And the command should not execute
    assert_eq!(status_of(root, "AUTH-001"), "specifying");
    assert!(!all.contains("status updated"), "{all}");
    // @step And the hook stderr should be wrapped in system-reminder tags
    assert!(
        all.contains("<system-reminder>\nHook: validate-feature-file"),
        "{all}"
    );
    // @step And the command should exit with non-zero code
    assert_eq!(output.status.code(), Some(1), "{all}");
}

#[test]
fn post_hook_runs_automation_after_command_succeeds() {
    // @step Given I have a non-blocking post-hook "run-tests" for "post-update-work-unit-status"
    let dir = project(
        json!({"post-update-work-unit-status": [{"name": "run-tests", "command": "spec/hooks/run-tests.sh"}]}),
        &[],
    );
    let root = dir.path();
    script(
        root,
        "spec/hooks/run-tests.sh",
        "echo 'Running tests...'\ncp spec/work-units.json seen-board.json",
    );
    // @step And work unit "AUTH-001" is being moved to implementing status
    assert_eq!(status_of(root, "AUTH-001"), "specifying");
    // @step When I run "fspec update-work-unit-status AUTH-001 implementing"
    let output = fspec(
        root,
        &["update-work-unit-status", "AUTH-001", "implementing"],
    );
    // @step Then the command should execute successfully
    assert!(output.status.success(), "{}", text(&output));
    // @step And the post-hook should execute after status change
    assert!(
        text(&output).contains("Running tests..."),
        "{}",
        text(&output)
    );
    // @step And the hook should see work unit "AUTH-001" saved with status "implementing"
    assert_eq!(
        status_in(&root.join("seen-board.json"), "AUTH-001"),
        "implementing"
    );
}

#[test]
fn blocking_pre_hook_failure_prevents_command_execution() {
    // @step Given I have a blocking pre-hook "lint" for "pre-update-work-unit-status"
    let dir = project_of(
        "story",
        json!({"pre-update-work-unit-status": [{"name": "lint", "command": "spec/hooks/lint.sh", "blocking": true}]}),
        &[],
    );
    let root = dir.path();
    // @step And the hook exits with code 1
    // @step And the hook outputs "Lint errors found" to stderr
    script(
        root,
        "spec/hooks/lint.sh",
        "echo 'Lint errors found' >&2\nexit 1",
    );
    // @step When I run "fspec update-work-unit-status AUTH-001 testing"
    let output = fspec(root, &["update-work-unit-status", "AUTH-001", "testing"]);
    let all = text(&output);
    // @step Then the pre-hook should execute and fail
    assert_eq!(output.status.code(), Some(1), "{all}");
    // @step And the command should not execute
    assert_eq!(status_of(root, "AUTH-001"), "specifying");
    // @step And the hook stderr should be wrapped in system-reminder tags
    let start = all
        .find("<system-reminder>")
        .unwrap_or_else(|| panic!("{all}"));
    let reminder = &all[start..all.find("</system-reminder>").unwrap()];
    // @step And the system-reminder should contain "Hook: lint"
    assert!(reminder.contains("Hook: lint"), "{reminder}");
    // @step And the system-reminder should contain "Exit code: 1"
    assert!(reminder.contains("Exit code: 1"), "{reminder}");
    // @step And the system-reminder should contain "Lint errors found"
    assert!(reminder.contains("Lint errors found"), "{reminder}");
}

#[test]
fn non_blocking_post_hook_failure_does_not_affect_command_success() {
    // @step Given I have a non-blocking post-hook "notify" for "post-update-work-unit-status"
    let dir = project(
        json!({"post-update-work-unit-status": [{"name": "notify", "command": "spec/hooks/notify.sh"}]}),
        &[],
    );
    let root = dir.path();
    // @step And the hook exits with code 1
    // @step And the hook outputs "Notification failed" to stderr
    script(
        root,
        "spec/hooks/notify.sh",
        "echo 'Notification failed' >&2\nexit 1",
    );
    // @step When I run "fspec update-work-unit-status AUTH-001 implementing"
    let output = fspec(
        root,
        &["update-work-unit-status", "AUTH-001", "implementing"],
    );
    let all = text(&output);
    // @step Then the command should execute successfully
    assert_eq!(status_of(root, "AUTH-001"), "implementing");
    // @step And the post-hook should execute after status change
    assert!(all.contains("Notification failed"), "{all}");
    // @step And the hook failure should not prevent command success
    assert_eq!(output.status.code(), Some(0), "{all}");
    // @step And the hook stderr should be displayed without system-reminder wrapping
    assert!(!all.contains("<system-reminder>\nHook: notify"), "{all}");
}

#[test]
fn hook_with_tag_condition_only_runs_for_matching_work_units() {
    // @step Given I have a hook with condition tags ["@security"]
    // @step And the hook is configured for "post-update-work-unit-status"
    // @step And work unit "AUTH-001" has tags ["@security", "@critical"]
    // @step And work unit "DASH-001" has tags ["@ui", "@critical"]
    let dir = project(
        json!({"post-update-work-unit-status": [
            {"name": "security", "command": "spec/hooks/security.sh", "condition": {"tags": ["@security"]}}
        ]}),
        &[
            ("AUTH-001", json!(["@security", "@critical"])),
            ("DASH-001", json!(["@ui", "@critical"])),
        ],
    );
    let root = dir.path();
    script(
        root,
        "spec/hooks/security.sh",
        "sed -n 's/.*\"workUnitId\":\"\\([^\"]*\\)\".*/\\1/p' >> security-runs.log",
    );
    // @step When I run "fspec update-work-unit-status AUTH-001 implementing"
    let first = fspec(
        root,
        &["update-work-unit-status", "AUTH-001", "implementing"],
    );
    assert!(first.status.success(), "{}", text(&first));
    // @step Then the hook should execute because work unit has @security tag
    let runs = fs::read_to_string(root.join("security-runs.log")).unwrap();
    assert_eq!(runs.lines().collect::<Vec<_>>(), vec!["AUTH-001"]);
    // @step When I run "fspec update-work-unit-status DASH-001 implementing"
    let second = fspec(
        root,
        &["update-work-unit-status", "DASH-001", "implementing"],
    );
    assert!(second.status.success(), "{}", text(&second));
    // @step Then the hook should not execute because work unit lacks @security tag
    let runs = fs::read_to_string(root.join("security-runs.log")).unwrap();
    assert_eq!(runs.lines().collect::<Vec<_>>(), vec!["AUTH-001"]);
}

#[test]
fn hook_output_formatted_with_system_reminder_for_blocking_failures() {
    // @step Given I have a blocking post-hook "validate" for "post-update-work-unit-status"
    let dir = project(
        json!({"post-update-work-unit-status": [
            {"name": "validate", "command": "spec/hooks/validate.sh", "blocking": true}
        ]}),
        &[],
    );
    let root = dir.path();
    // @step And the hook exits with code 1
    // @step And the hook outputs "Validation failed: tests not passing" to stderr
    script(
        root,
        "spec/hooks/validate.sh",
        "echo 'Validation failed: tests not passing' >&2\nexit 1",
    );
    // @step When I run "fspec update-work-unit-status AUTH-001 implementing"
    let output = fspec(
        root,
        &["update-work-unit-status", "AUTH-001", "implementing"],
    );
    let all = text(&output);
    // @step Then the command should execute
    assert_eq!(status_of(root, "AUTH-001"), "implementing");
    // @step And the post-hook should execute and fail
    assert_eq!(output.status.code(), Some(1), "{all}");
    // @step And the hook stderr should be wrapped in system-reminder tags
    // @step And the output should contain "<system-reminder>"
    assert!(all.contains("<system-reminder>"), "{all}");
    // @step And the output should contain "Hook: validate"
    assert!(all.contains("Hook: validate"), "{all}");
    // @step And the output should contain "Exit code: 1"
    assert!(all.contains("Exit code: 1"), "{all}");
    // @step And the output should contain "Validation failed: tests not passing"
    assert!(
        all.contains("Validation failed: tests not passing"),
        "{all}"
    );
    // @step And the output should contain "</system-reminder>"
    assert!(all.contains("</system-reminder>"), "{all}");
}

#[test]
fn agent_fspec_tool_calls_run_command_hooks() {
    // @step Given I have a non-blocking post-hook "notify" for "post-update-work-unit-status"
    let dir = project(
        json!({"post-update-work-unit-status": [{"name": "notify", "command": "spec/hooks/notify.sh"}]}),
        &[],
    );
    let root = dir.path();
    // @step And the hook outputs "Notified" to stdout
    script(root, "spec/hooks/notify.sh", "echo Notified");
    // @step When an agent fspec tool call runs "update-work-unit-status" for "AUTH-001" with status "implementing"
    let result = dispatch_command(DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: json!({"workUnitId": "AUTH-001", "status": "implementing"}).to_string(),
        project_root: root.to_path_buf(),
    });
    // @step Then the tool call should succeed
    assert!(result.success, "{:?}", result.error);
    assert_eq!(status_of(root, "AUTH-001"), "implementing");
    // @step And the tool call system reminder should contain "Notified"
    let reminder = result.system_reminder.unwrap_or_default();
    assert!(reminder.contains("Notified"), "{reminder:?}");
}

#[test]
fn a_cli_command_routed_through_the_dispatcher_runs_its_hooks_once() {
    // @step Given I have a non-blocking post-hook "count" for "post-add-bounded-context" that appends a line to "hook-runs.log"
    let dir = project(
        json!({"post-add-bounded-context": [{"name": "count", "command": "echo ran >> hook-runs.log"}]}),
        &[],
    );
    let root = dir.path();
    // @step When I run "fspec add-bounded-context AUTH-001 Billing"
    let output = fspec(root, &["add-bounded-context", "AUTH-001", "Billing"]);
    assert!(output.status.success(), "{}", text(&output));
    // @step Then "hook-runs.log" should contain exactly one line
    let runs = fs::read_to_string(root.join("hook-runs.log")).unwrap();
    assert_eq!(runs.lines().collect::<Vec<_>>(), vec!["ran"]);
}

#[test]
fn a_broken_hook_configuration_fails_commands_but_not_the_hook_management_commands() {
    // @step Given spec/fspec-hooks.json holds invalid JSON
    let dir = project(json!({}), &[]);
    let root = dir.path();
    fs::write(
        root.join("spec/fspec-hooks.json"),
        "{ \"hooks\": { invalid json }",
    )
    .unwrap();
    // @step When I run "fspec validate-hooks"
    let validate = fspec(root, &["validate-hooks"]);
    // @step Then the output should contain "spec/fspec-hooks.json could not be parsed"
    assert!(
        text(&validate).contains("spec/fspec-hooks.json could not be parsed"),
        "{}",
        text(&validate)
    );
    // @step When I run "fspec list-work-units"
    let list = fspec(root, &["list-work-units"]);
    // @step Then the command should exit with non-zero code
    assert_eq!(list.status.code(), Some(1), "{}", text(&list));
    // @step And the output should contain "Invalid JSON in spec/fspec-hooks.json"
    assert!(
        text(&list).contains("Invalid JSON in spec/fspec-hooks.json"),
        "{}",
        text(&list)
    );
}

#[test]
fn a_hook_of_another_event_does_not_affect_a_command() {
    // @step Given I have a hook for "post-implementing" whose script "spec/hooks/later.sh" does not exist yet
    let dir = project(
        json!({"post-implementing": [{"name": "later", "command": "spec/hooks/later.sh", "blocking": true}]}),
        &[],
    );
    let root = dir.path();
    assert!(!root.join("spec/hooks/later.sh").exists());
    // @step When I run "fspec update-work-unit-status AUTH-001 implementing"
    let output = fspec(
        root,
        &["update-work-unit-status", "AUTH-001", "implementing"],
    );
    // @step Then the command should execute successfully
    assert!(output.status.success(), "{}", text(&output));
    assert_eq!(status_of(root, "AUTH-001"), "implementing");
}

#[test]
fn a_failed_command_skips_its_post_hooks() {
    // @step Given I have a non-blocking post-hook "notify" for "post-update-work-unit-status" that appends a line to "hook-runs.log"
    let dir = project(
        json!({"post-update-work-unit-status": [{"name": "notify", "command": "echo ran >> hook-runs.log"}]}),
        &[],
    );
    let root = dir.path();
    // @step When I run "fspec update-work-unit-status AUTH-001 done"
    let output = fspec(root, &["update-work-unit-status", "AUTH-001", "done"]);
    // @step Then the command should exit with non-zero code
    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
    // @step And "hook-runs.log" should not exist
    assert!(!root.join("hook-runs.log").exists());
}

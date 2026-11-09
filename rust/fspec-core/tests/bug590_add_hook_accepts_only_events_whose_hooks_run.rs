//! Feature: spec/features/add-hook-accepts-only-events-whose-hooks-run.feature
//!
//! BUG-590: add-hook wrote a hook for any event name, so an event that no
//! fspec command or status change ever raises stored a hook that never
//! runs. add-hook now accepts only pre-/post-<command> of a command that
//! runs command hooks and pre-/post-<status> of a workflow status, and
//! rejects anything else before writing.

use std::path::Path;

use codelet_fspec_core::command_hooks::check_hook_event;
use codelet_fspec_core::help::configs::add_hook::CONFIG;
use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const UNKNOWN_PRE_LINT: &str = "unknown hook event \
     'pre-lint': a global hook runs on pre-<command> or post-<command> of an fspec command \
     that runs command hooks, or on pre-<status> or post-<status> of a workflow status \
     (backlog, specifying, testing, implementing, validating, done, blocked)";

const STATUS_NOTE: &str = "A pre-<status> or post-<status> hook runs for every work unit on \
     that status change; to add a hook for one work unit only, use fspec add-virtual-hook";

/// Run `fspec add-hook <event> <name> --command <command>` through the
/// dispatcher both front doors share; the error text on failure.
fn add_hook(root: &Path, event: &str, name: &str, command: &str) -> Result<(), String> {
    let result = dispatch_command(DispatchRequest {
        command: "add-hook".to_string(),
        args_json: json!({ "event": event, "name": name, "command": command, "blocking": false })
            .to_string(),
        project_root: root.to_path_buf(),
    });
    if result.success {
        Ok(())
    } else {
        Err(result.error.unwrap_or_default())
    }
}

fn hooks_file(root: &Path) -> std::path::PathBuf {
    root.join("spec").join("fspec-hooks.json")
}

fn hook_names(root: &Path, event: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(hooks_file(root))?)?;
    let hooks = config["hooks"][event]
        .as_array()
        .ok_or_else(|| format!("no hooks under {event}: {config}"))?;
    Ok(hooks
        .iter()
        .filter_map(|hook| hook["name"].as_str().map(str::to_string))
        .collect())
}

#[test]
fn a_hook_on_an_fspec_command_event_is_added() -> TestResult {
    // @step Given a project without spec/fspec-hooks.json
    let project = tempfile::tempdir()?;
    std::fs::create_dir_all(project.path().join("spec"))?;

    // @step When I run "fspec add-hook pre-update-work-unit-status check-ready --command spec/hooks/check.sh"
    add_hook(
        project.path(),
        "pre-update-work-unit-status",
        "check-ready",
        "spec/hooks/check.sh",
    )?;

    // @step Then spec/fspec-hooks.json holds the hook "check-ready" under the event "pre-update-work-unit-status"
    assert_eq!(
        hook_names(project.path(), "pre-update-work-unit-status")?,
        vec!["check-ready"]
    );
    Ok(())
}

#[test]
fn a_hook_on_a_workflow_status_event_is_added_as_a_global_status_hook() -> TestResult {
    // @step Given a project without spec/fspec-hooks.json
    let project = tempfile::tempdir()?;
    std::fs::create_dir_all(project.path().join("spec"))?;

    // @step When I run "fspec add-hook pre-implementing lint --command spec/hooks/lint.sh"
    add_hook(
        project.path(),
        "pre-implementing",
        "lint",
        "spec/hooks/lint.sh",
    )?;

    // @step Then spec/fspec-hooks.json holds the hook "lint" under the event "pre-implementing"
    assert_eq!(
        hook_names(project.path(), "pre-implementing")?,
        vec!["lint"]
    );
    Ok(())
}

#[test]
fn an_event_that_is_neither_a_command_nor_a_status_is_rejected_before_writing() -> TestResult {
    // @step Given spec/fspec-hooks.json holds one hook under "pre-implementing"
    let project = tempfile::tempdir()?;
    std::fs::create_dir_all(project.path().join("spec"))?;
    let before = "{\n  \"hooks\": {\n    \"pre-implementing\": [\n      { \"name\": \"lint\", \
                  \"command\": \"lint.sh\", \"blocking\": false }\n    ]\n  }\n}\n";
    std::fs::write(hooks_file(project.path()), before)?;

    // @step When I run "fspec add-hook pre-lint check --command x.sh"
    let result = add_hook(project.path(), "pre-lint", "check", "x.sh");

    // @step Then it fails with "unknown hook event 'pre-lint': a global hook runs on pre-<command> or post-<command> of an fspec command that runs command hooks, or on pre-<status> or post-<status> of a workflow status (backlog, specifying, testing, implementing, validating, done, blocked)"
    assert_eq!(result.err().as_deref(), Some(UNKNOWN_PRE_LINT));

    // @step And spec/fspec-hooks.json is byte-for-byte unchanged
    assert_eq!(std::fs::read_to_string(hooks_file(project.path()))?, before);
    Ok(())
}

#[test]
fn an_event_of_a_command_that_runs_no_command_hooks_is_rejected() -> TestResult {
    // @step Given a project without spec/fspec-hooks.json
    let project = tempfile::tempdir()?;
    std::fs::create_dir_all(project.path().join("spec"))?;

    // @step When I run "fspec add-hook post-add-hook audit --command x.sh"
    let result = add_hook(project.path(), "post-add-hook", "audit", "x.sh");

    // @step Then it fails with the unknown hook event error naming "post-add-hook"
    assert_eq!(
        result.err().as_deref(),
        Some(
            UNKNOWN_PRE_LINT
                .replace("'pre-lint'", "'post-add-hook'")
                .as_str()
        )
    );

    // @step And no spec/fspec-hooks.json is written
    assert!(!hooks_file(project.path()).exists());
    Ok(())
}

/// The event of an `fspec add-hook <event> ...` example.
fn example_event(command: &str) -> Option<&str> {
    command
        .strip_prefix("fspec add-hook ")?
        .split_whitespace()
        .next()
}

#[test]
fn add_hooks_help_names_only_accepted_events_and_points_to_add_virtual_hook_for_one_work_unit(
) -> TestResult {
    // @step Given add-hook's help config
    let config = CONFIG;

    // @step When every event in its examples and common patterns is checked
    let mut commands: Vec<&str> = config.examples.iter().map(|e| e.command).collect();
    for pattern in config.common_patterns {
        if let codelet_fspec_core::help::CommonPatternEntry::Structured(p) = pattern {
            commands.push(p.example);
        }
    }
    let events: Vec<&str> = commands.iter().filter_map(|c| example_event(c)).collect();
    assert_eq!(events.len(), commands.len(), "{commands:?}");

    // @step Then every event is accepted by add-hook
    let rejected: Vec<String> = events
        .iter()
        .filter_map(|event| check_hook_event(event).err())
        .collect();
    assert!(rejected.is_empty(), "{rejected:?}");

    // @step And its notes say a status hook runs for every work unit and a hook on one work unit is added with add-virtual-hook
    assert!(config.notes.contains(&STATUS_NOTE), "{:?}", config.notes);
    Ok(())
}

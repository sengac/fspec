#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/hook-execution-engine.feature
//
// HOOK-018: command hooks run as child processes with the JSON context on
// stdin, their output captured, a per-hook timeout, and blocking failures
// halting the phase (codelet_fspec_core::command_hooks).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use codelet_fspec_core::command_hooks::{
    format_hook_output, run_event_hooks, HookPhase, PhaseOutcome,
};
use serde_json::{json, Value};
use tempfile::TempDir;

fn script(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn hooks(root: &Path, event: &str, hooks: Value) {
    fs::create_dir_all(root.join("spec")).unwrap();
    let config = json!({"hooks": {event: hooks}});
    fs::write(root.join("spec/fspec-hooks.json"), config.to_string()).unwrap();
}

fn run(root: &Path, event: &str, work_unit: Option<&str>, phase: HookPhase) -> PhaseOutcome {
    run_event_hooks(root, event, work_unit, phase).unwrap_or_else(|e| panic!("hooks run: {e}"))
}

#[test]
fn execute_single_non_blocking_hook_that_succeeds() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a hook configuration with a non-blocking hook "lint" for "post-implementing"
    hooks(
        dir.path(),
        "post-implementing",
        json!([{"name": "lint", "command": "spec/hooks/lint.sh"}]),
    );
    // @step And the hook script "spec/hooks/lint.sh" exits with code 0
    script(
        dir.path(),
        "spec/hooks/lint.sh",
        "echo \"lint pid $$\"\nexit 0",
    );
    // @step When I execute the hook for event "post-implementing"
    let outcome = run(dir.path(), "post-implementing", None, HookPhase::Post);
    // @step Then the hook should be spawned as a child process
    let result = &outcome.results[0];
    assert!(result.stdout.starts_with("lint pid "), "{result:?}");
    assert_ne!(
        result.stdout.trim(),
        format!("lint pid {}", std::process::id())
    );
    // @step And the hook stdout should be captured and displayed
    assert!(outcome.output.contains("lint pid "), "{}", outcome.output);
    // @step And the hook should complete successfully
    assert!(result.success());
    assert_eq!(result.exit_code, Some(0));
    // @step And the command should continue execution
    assert!(!outcome.blocked);
}

#[test]
fn execute_single_non_blocking_hook_that_fails() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a hook configuration with a non-blocking hook "check" for "post-testing"
    hooks(
        dir.path(),
        "post-testing",
        json!([{"name": "check", "command": "spec/hooks/check.sh"}]),
    );
    // @step And the hook script "spec/hooks/check.sh" exits with code 1
    script(dir.path(), "spec/hooks/check.sh", "echo checking\nexit 1");
    // @step When I execute the hook for event "post-testing"
    let outcome = run(dir.path(), "post-testing", None, HookPhase::Post);
    // @step Then the hook should be spawned as a child process
    assert_eq!(outcome.results[0].stdout.trim(), "checking");
    // @step And a warning should be displayed about the hook failure
    assert!(
        outcome.output.contains("Hook 'check' failed (exit code 1)"),
        "{}",
        outcome.output
    );
    assert!(!outcome.output.contains("<system-reminder>"));
    // @step And the command should continue execution despite the failure
    assert!(!outcome.blocked);
}

#[test]
fn execute_blocking_hook_that_fails() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a hook configuration with a blocking hook "validate" for "post-implementing"
    hooks(
        dir.path(),
        "post-implementing",
        json!([{"name": "validate", "command": "spec/hooks/validate.sh", "blocking": true}]),
    );
    // @step And the hook script "spec/hooks/validate.sh" exits with code 1
    script(
        dir.path(),
        "spec/hooks/validate.sh",
        "echo invalid >&2\nexit 1",
    );
    // @step When I execute the hook for event "post-implementing"
    let outcome = run(dir.path(), "post-implementing", None, HookPhase::Post);
    // @step Then the hook should be spawned as a child process
    assert_eq!(outcome.results[0].exit_code, Some(1));
    // @step And an error should be displayed about the hook failure
    assert!(
        outcome.output.contains("<system-reminder>"),
        "{}",
        outcome.output
    );
    assert!(
        outcome.output.contains("Hook: validate"),
        "{}",
        outcome.output
    );
    // @step And the command execution should be halted
    assert!(outcome.blocked);
    // @step And the fspec command should exit with non-zero code
    // A blocked phase is what makes both front doors fail the command; the
    // CLI exit code itself is proven in integrate-hooks-into-all-commands.
    assert!(!outcome.results[0].success());
}

#[test]
fn hook_times_out_after_configured_duration() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a hook configuration with a hook "slow-test" with timeout 1 seconds
    hooks(
        dir.path(),
        "post-testing",
        json!([{"name": "slow-test", "command": "spec/hooks/slow-test.sh", "timeout": 1}]),
    );
    // @step And the hook script "spec/hooks/slow-test.sh" never exits
    script(
        dir.path(),
        "spec/hooks/slow-test.sh",
        "echo $$ > hook.pid\nexec tail -f /dev/null",
    );
    // @step When I execute the hook for event "post-testing"
    let outcome = run(dir.path(), "post-testing", None, HookPhase::Post);
    let result = &outcome.results[0];
    // @step Then the hook process should be killed
    // The script never exits, so returning at all means the executor killed
    // and reaped it; when it got as far as recording its pid, that pid is gone.
    assert!(result.timed_out, "{result:?}");
    if let Ok(pid) = fs::read_to_string(dir.path().join("hook.pid")) {
        let alive = std::process::Command::new("kill")
            .args(["-0", pid.trim()])
            .status()
            .unwrap()
            .success();
        assert!(!alive, "hook process {pid} still alive");
    }
    // @step And a timeout error should be displayed
    assert!(
        format_hook_output(result).contains("Hook 'slow-test' timed out after 1s"),
        "{}",
        format_hook_output(result)
    );
    // @step And the hook should be marked as failed
    assert!(!result.success());
    assert_eq!(result.exit_code, None);
}

#[test]
fn hook_receives_context_via_stdin() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a hook configuration with a hook "notify" for "post-implementing"
    hooks(
        dir.path(),
        "post-implementing",
        json!([{"name": "notify", "command": "spec/hooks/notify.sh"}]),
    );
    script(dir.path(), "spec/hooks/notify.sh", "cat > context.json");
    // @step And I am executing in context of work unit "AUTH-001"
    let work_unit = Some("AUTH-001");
    // @step When I execute the hook for event "post-implementing"
    let outcome = run(dir.path(), "post-implementing", work_unit, HookPhase::Post);
    assert!(outcome.results[0].success(), "{:?}", outcome.results);
    // @step Then the hook should receive JSON context via stdin
    let raw = fs::read_to_string(dir.path().join("context.json")).unwrap();
    let context: Value = serde_json::from_str(&raw).unwrap();
    // @step And the context should include "workUnitId" field with value "AUTH-001"
    assert_eq!(context["workUnitId"], "AUTH-001");
    // @step And the context should include "event" field with value "post-implementing"
    assert_eq!(context["event"], "post-implementing");
    // @step And the context should include "timestamp" field with ISO 8601 format
    let timestamp = context["timestamp"].as_str().unwrap();
    assert!(
        chrono::DateTime::parse_from_rfc3339(timestamp).is_ok(),
        "{timestamp}"
    );
}

#[test]
fn multiple_hooks_execute_sequentially() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a hook configuration with hooks "first" and "second" for "post-implementing"
    // @step And both hooks are configured to execute in order
    hooks(
        dir.path(),
        "post-implementing",
        json!([
            {"name": "first", "command": "spec/hooks/first.sh"},
            {"name": "second", "command": "spec/hooks/second.sh"}
        ]),
    );
    script(
        dir.path(),
        "spec/hooks/first.sh",
        "echo first-start >> order.log\necho first-end >> order.log",
    );
    script(
        dir.path(),
        "spec/hooks/second.sh",
        "echo second-start >> order.log\necho second-end >> order.log",
    );
    // @step When I execute the hooks for event "post-implementing"
    let outcome = run(dir.path(), "post-implementing", None, HookPhase::Post);
    let log = fs::read_to_string(dir.path().join("order.log")).unwrap();
    let lines: Vec<&str> = log.lines().collect();
    // @step Then hook "first" should start execution
    assert_eq!(lines.first(), Some(&"first-start"));
    // @step And hook "first" should complete before hook "second" starts
    // @step And hook "second" should start execution after hook "first" completes
    // @step And hooks should execute in sequential order
    assert_eq!(
        lines,
        vec!["first-start", "first-end", "second-start", "second-end"]
    );
    let names: Vec<&str> = outcome
        .results
        .iter()
        .map(|r| r.hook_name.as_str())
        .collect();
    assert_eq!(names, vec!["first", "second"]);
}

#[test]
fn hook_stdout_and_stderr_are_captured_and_displayed() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a hook configuration with a hook "test" for "post-testing"
    hooks(
        dir.path(),
        "post-testing",
        json!([{"name": "test", "command": "spec/hooks/test.sh"}]),
    );
    // @step And the hook script outputs "Running tests..." to stdout
    // @step And the hook script outputs "Warning: deprecated API" to stderr
    script(
        dir.path(),
        "spec/hooks/test.sh",
        "echo 'Running tests...'\necho 'Warning: deprecated API' >&2",
    );
    // @step When I execute the hook for event "post-testing"
    let outcome = run(dir.path(), "post-testing", None, HookPhase::Post);
    let result = &outcome.results[0];
    // @step Then the stdout message "Running tests..." should be captured
    assert_eq!(result.stdout.trim(), "Running tests...");
    // @step And the stderr message "Warning: deprecated API" should be captured
    assert_eq!(result.stderr.trim(), "Warning: deprecated API");
    // @step And both stdout and stderr should be displayed to the user
    assert!(
        outcome.output.contains("Running tests..."),
        "{}",
        outcome.output
    );
    assert!(
        outcome.output.contains("Warning: deprecated API"),
        "{}",
        outcome.output
    );
}

#[test]
fn script_path_with_arguments_runs_through_the_shell() {
    let dir = TempDir::new().unwrap();
    // @step Given I have a hook configuration with a hook "record" for "post-testing" whose command is "spec/hooks/record.sh enter"
    hooks(
        dir.path(),
        "post-testing",
        json!([{"name": "record", "command": "spec/hooks/record.sh enter"}]),
    );
    // @step And the script "spec/hooks/record.sh" writes its first argument to a file
    script(
        dir.path(),
        "spec/hooks/record.sh",
        "echo \"$1\" > recorded.txt",
    );
    // @step When I execute the hook for event "post-testing"
    let outcome = run(dir.path(), "post-testing", None, HookPhase::Post);
    // @step Then the hook should complete successfully
    assert!(outcome.results[0].success(), "{:?}", outcome.results);
    // @step And the file should contain "enter"
    let recorded = fs::read_to_string(dir.path().join("recorded.txt")).unwrap();
    assert_eq!(recorded.trim(), "enter");
}

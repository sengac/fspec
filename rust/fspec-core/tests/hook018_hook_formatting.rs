#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/system-reminder-formatting-for-ai.feature
//
// HOOK-018: a command hook's result is formatted for people and AI agents;
// failing blocking hooks are wrapped in a <system-reminder>
// (codelet_fspec_core::command_hooks::format_hook_output).

use codelet_fspec_core::command_hooks::{format_hook_output, HookResult};

fn result(name: &str, blocking: bool, exit_code: i32, stdout: &str, stderr: &str) -> HookResult {
    HookResult {
        hook_name: name.to_string(),
        blocking,
        exit_code: Some(exit_code),
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
        timed_out: false,
        timeout_secs: 60,
    }
}

/// The text between the first <system-reminder> and </system-reminder>.
fn reminder(formatted: &str) -> &str {
    let start = formatted
        .find("<system-reminder>")
        .unwrap_or_else(|| panic!("no system-reminder in {formatted:?}"));
    let end = formatted.find("</system-reminder>").unwrap();
    &formatted[start..end]
}

#[test]
fn blocking_hook_fails_with_stderr_wrapped_in_system_reminder() {
    // @step Given I have a blocking hook that fails with exit code 1
    // @step And the hook outputs "Invalid config" to stderr
    let hook = result("validate", true, 1, "", "Invalid config\n");
    // @step When I format the hook output for display
    let formatted = format_hook_output(&hook);
    // @step Then the stderr should be wrapped in system-reminder tags
    let inside = reminder(&formatted);
    assert!(
        formatted.trim_end().ends_with("</system-reminder>"),
        "{formatted}"
    );
    // @step And the system-reminder should include the hook name
    assert!(inside.contains("Hook: validate"), "{inside}");
    // @step And the system-reminder should include the exit code
    assert!(inside.contains("Exit code: 1"), "{inside}");
    // @step And the system-reminder should include the stderr content
    assert!(inside.contains("Invalid config"), "{inside}");
}

#[test]
fn non_blocking_hook_stderr_displayed_as_is() {
    // @step Given I have a non-blocking hook that fails with exit code 1
    // @step And the hook outputs "Warning: deprecated API" to stderr
    let hook = result("lint", false, 1, "", "Warning: deprecated API\n");
    // @step When I format the hook output for display
    let formatted = format_hook_output(&hook);
    // @step Then the stderr should be displayed as-is
    assert!(
        formatted.lines().any(|l| l == "Warning: deprecated API"),
        "{formatted}"
    );
    // @step And the stderr should not be wrapped in system-reminder tags
    assert!(!formatted.contains("<system-reminder>"), "{formatted}");
}

#[test]
fn blocking_hook_succeeds_with_no_system_reminder() {
    // @step Given I have a blocking hook that succeeds with exit code 0
    // @step And the hook outputs "All checks passed" to stdout
    let hook = result("validate", true, 0, "All checks passed\n", "");
    // @step When I format the hook output for display
    let formatted = format_hook_output(&hook);
    // @step Then no system-reminder should be generated
    assert!(!formatted.contains("<system-reminder>"), "{formatted}");
    // @step And only stdout should be displayed
    assert_eq!(formatted, "All checks passed");
}

#[test]
fn blocking_hook_fails_with_empty_stderr_still_produces_a_system_reminder() {
    // @step Given I have a blocking hook named "test" that fails with exit code 1
    // @step And the hook produces no stderr output
    let hook = result("test", true, 1, "", "");
    // @step When I format the hook output for display
    let formatted = format_hook_output(&hook);
    let inside = reminder(&formatted);
    // @step Then the system-reminder should contain "Hook: test"
    assert!(inside.contains("Hook: test"), "{inside}");
    // @step And the system-reminder should contain "Exit code: 1"
    assert!(inside.contains("Exit code: 1"), "{inside}");
    // @step And the system-reminder should contain "(Hook failed with no error output)"
    assert!(
        inside.contains("(Hook failed with no error output)"),
        "{inside}"
    );
}

#[test]
fn system_reminder_includes_hook_metadata() {
    // @step Given I have a blocking hook named "validate" that fails
    // @step And the hook exits with code 1
    // @step And the hook outputs "Validation failed: missing field 'name'" to stderr
    let hook = result(
        "validate",
        true,
        1,
        "",
        "Validation failed: missing field 'name'",
    );
    // @step When I format the hook output for display
    let formatted = format_hook_output(&hook);
    let inside = reminder(&formatted);
    // @step Then the system-reminder should contain "Hook: validate"
    assert!(inside.contains("Hook: validate"), "{inside}");
    // @step And the system-reminder should contain "Exit code: 1"
    assert!(inside.contains("Exit code: 1"), "{inside}");
    // @step And the system-reminder should contain "Validation failed: missing field 'name'"
    assert!(
        inside.contains("Validation failed: missing field 'name'"),
        "{inside}"
    );
}

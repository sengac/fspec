#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/tool-call-arg-errors-include-recovery-guidance.feature
//
// This test file validates the acceptance criteria for TOOL-023: fspec
// arg-related tool-call failures must be self-explanatory. Scenarios map
// directly to the Gherkin scenarios in the feature file above; every Gherkin
// step is mirrored by an @step comment.

use codelet_fspec_core::canonical::suggest_command;
use codelet_fspec_core::{dispatch_command, DispatchRequest};
use tempfile::TempDir;

fn dispatch(command: &str, args_json: &str, project_root: &std::path::Path) -> String {
    let result = dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    });
    assert!(!result.success, "expected success=false for {command}");
    result
        .error
        .unwrap_or_else(|| "(no error message)".to_string())
}

/// Scenario: Unknown fspec command error suggests the closest canonical command
#[test]
fn scenario_unknown_fs_command_suggests_closest_canonical_command() {
    let tmp = TempDir::new().expect("temp dir");

    // @step Given I dispatch the Fspec command "list-workunit"
    // @step When the command name is not canonical
    let msg = dispatch("list-workunit", "{}", tmp.path());

    // @step Then the dispatcher returns success=false
    // (asserted inside the dispatch helper)

    // @step And the error message contains the substring "Unknown fspec command: list-workunit"
    assert!(
        msg.contains("Unknown fspec command: list-workunit"),
        "missing UnknownCommand line in: {msg}"
    );

    // @step And the error message contains a line starting with "Did you mean: list-work-units"
    let line = msg
        .lines()
        .find(|l| l.starts_with("Did you mean:"))
        .unwrap_or_else(|| panic!("missing 'Did you mean:' line in: {msg}"));
    assert_eq!(
        line, "Did you mean: list-work-units",
        "wrong suggestion in: {msg}"
    );

    // @step And the error message tells me to re-call with the suggested name
    assert!(
        msg.contains("Re-call with the suggested command name"),
        "missing re-call guidance in: {msg}"
    );

    // @step And the error message points me to "<command> --help" for full argument reference
    assert!(msg.contains("--help"), "missing --help pointer in: {msg}");
}

/// Scenario: Unknown fspec command with no close match gets no suggestion line
#[test]
fn scenario_unknown_fs_command_with_no_close_match_gets_no_suggestion_line() {
    let tmp = TempDir::new().expect("temp dir");

    // @step Given I dispatch the Fspec command "zzzqqqxx"
    // @step When the command name is not canonical and closely matches no command
    let msg = dispatch("zzzqqqxx", "{}", tmp.path());

    // @step Then the dispatcher returns success=false
    // (asserted inside the dispatch helper)

    // @step And the error message contains the substring "Unknown fspec command: zzzqqqxx"
    assert!(
        msg.contains("Unknown fspec command: zzzqqqxx"),
        "missing UnknownCommand line in: {msg}"
    );

    // @step And the error message does NOT contain the substring "Did you mean"
    assert!(
        !msg.contains("Did you mean"),
        "unexpected suggestion for gibberish input: {msg}"
    );
}

/// Scenario: Unknown fspec command suggests the Rust-only foundation-status extension
#[test]
fn scenario_unknown_fs_command_suggests_foundation_status_extension() {
    let tmp = TempDir::new().expect("temp dir");

    // @step Given I dispatch the Fspec command "foundations-status"
    // @step When the command name is not canonical but closely matches foundation-status
    let msg = dispatch("foundations-status", "{}", tmp.path());

    // @step Then the error message contains a line starting with "Did you mean: foundation-status"
    let line = msg
        .lines()
        .find(|l| l.starts_with("Did you mean:"))
        .unwrap_or_else(|| panic!("missing 'Did you mean:' line in: {msg}"));
    assert_eq!(
        line, "Did you mean: foundation-status",
        "wrong suggestion in: {msg}"
    );
}

/// Scenario: suggest_command returns the closest name for small typos
#[test]
fn scenario_suggest_command_returns_closest_name_for_small_typos() {
    // @step Given I ask the fuzzy matcher for "list-work-unitt"
    // @step Then it returns the suggestion "list-work-units"
    assert_eq!(suggest_command("list-work-unitt"), Some("list-work-units"));

    // @step And I ask the fuzzy matcher for "show-work-unit"
    // @step And it returns the suggestion "show-work-unit" (exact match on a real command)
    assert_eq!(suggest_command("show-work-unit"), Some("show-work-unit"));

    // @step And I ask the fuzzy matcher for "zzzqqqxx"
    // @step And it returns no suggestion
    assert_eq!(suggest_command("zzzqqqxx"), None);
}

/// Scenario: suggest_command ignores empty or oversized input
#[test]
fn scenario_suggest_command_ignores_empty_or_oversized_input() {
    // @step Given I ask the fuzzy matcher for ""
    // @step Then it returns no suggestion
    assert_eq!(suggest_command(""), None);

    // @step And I ask the fuzzy matcher for a 200-character gibberish string
    // @step And it returns no suggestion for the gibberish input
    let gibberish: String = "x".repeat(200);
    assert_eq!(suggest_command(&gibberish), None);
}

/// Scenario: Malformed inner args on a ported command include a usage hint
#[test]
fn scenario_malformed_inner_args_on_ported_command_include_usage_hint() {
    let tmp = TempDir::new().expect("temp dir");

    // @step Given I dispatch the ported command "list-work-units" with malformed inner args "{"
    // @step When the dispatcher parses the inner args
    let msg = dispatch("list-work-units", "{", tmp.path());

    // @step Then the dispatcher returns success=false
    // (asserted inside the dispatch helper)

    // @step And the error message contains the substring "Invalid args for fspec command list-work-units"
    assert!(
        msg.contains("Invalid args for fspec command list-work-units"),
        "missing InvalidArgs framing in: {msg}"
    );

    // @step And the error message contains the substring "failed to parse args"
    assert!(
        msg.contains("failed to parse args"),
        "missing parse-failure reason in: {msg}"
    );

    // @step And the error message tells me args must be a JSON object string
    assert!(
        msg.contains("args must be a JSON object string"),
        "missing args-shape guidance in: {msg}"
    );

    // @step And the error message points me to "list-work-units --help" for the full argument reference
    assert!(
        msg.contains("list-work-units --help"),
        "missing per-command help pointer in: {msg}"
    );
}

/// Scenario: Missing required inner field on a ported command includes a usage hint
#[test]
fn scenario_missing_required_inner_field_includes_usage_hint() {
    let tmp = TempDir::new().expect("temp dir");

    // @step Given I dispatch the ported command "show-work-unit" with inner args "{}"
    // @step When the dispatcher parses the inner args and finds the required field missing
    let msg = dispatch("show-work-unit", "{}", tmp.path());

    // @step Then the dispatcher returns success=false
    // (asserted inside the dispatch helper)

    // @step And the error message contains the substring "Invalid args for fspec command show-work-unit"
    assert!(
        msg.contains("Invalid args for fspec command show-work-unit"),
        "missing InvalidArgs framing in: {msg}"
    );

    // @step And the error message names the missing argument "workUnitId"
    assert!(
        msg.contains("workUnitId"),
        "missing required-argument name in: {msg}"
    );

    // @step And the error message points me to "show-work-unit --help" for the full argument reference
    assert!(
        msg.contains("show-work-unit --help"),
        "missing per-command help pointer in: {msg}"
    );
}

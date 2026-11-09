#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::ast_research_block;
use super::specifying::specifying_reminder;

// Feature: spec/features/specifying-reminder-names-ast-search-tool-per-mode.feature

#[test]
#[serial_test::serial(capture_mode)]
fn specifying_reminder_cli_mode_points_at_fspec_astgrep() {
    // @step Given a work unit exists in specifying-relevant state and FSPEC_CAPTURE_MODE is not set
    // @step When I dispatch update-work-unit-status for it with status "specifying"
    let tmp = tempfile::TempDir::new().unwrap();
    if crate::utils::mode::in_capture_mode() {
        return; // environment explicitly opts in; nothing to assert
    }
    let reminder = specifying_reminder("AUTH-001", tmp.path());
    // @step Then the specifying reminder in the response references `fspec astgrep`
    assert!(reminder.contains("fspec astgrep"));
    // @step And the specifying reminder does not reference `fspec research --tool=ast`
    assert!(!reminder.contains("fspec research --tool=ast"));
    assert!(!reminder.contains("AstGrep(language="));
}

#[test]
#[serial_test::serial(capture_mode)]
fn specifying_reminder_capture_mode_points_at_astgrep_tool() {
    // @step Given a work unit exists in specifying-relevant state and FSPEC_CAPTURE_MODE is set to "1"
    // @step When I dispatch update-work-unit-status for it with status "specifying"
    let tmp = tempfile::TempDir::new().unwrap();
    std::env::set_var("FSPEC_CAPTURE_MODE", "1");
    let reminder = specifying_reminder("AUTH-001", tmp.path());
    std::env::remove_var("FSPEC_CAPTURE_MODE");
    // @step Then the specifying reminder in the response references the AstGrep tool
    assert!(reminder.contains("AstGrep(language="));
    // @step And the specifying reminder does not reference `fspec astgrep`
    assert!(!reminder.contains("fspec astgrep"));
}

#[test]
fn ast_research_block_variants_are_mode_specific() {
    // @step Given both capture and CLI execution contexts
    // @step Then each names only its own tool
    let cli = ast_research_block(false);
    let harness = ast_research_block(true);
    assert!(cli.contains("fspec astgrep"));
    assert!(!cli.contains("AstGrep("));
    assert!(harness.contains("AstGrep and AstGrepRefactor"));
    assert!(!harness.contains("fspec astgrep"));
}

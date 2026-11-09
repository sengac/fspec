use super::review::{ast_research_error_message, is_in_capture_mode};

// Feature: spec/features/update-work-unit-status-rust-port.feature
//
// Locks in the capture-mode AST-research error variant, mirroring
// `buildASTResearchErrorMessage()` in `src/utils/review-validation.ts`.

#[test]
fn ast_research_message_cli_variant_directs_to_astgrep_command() {
    // @step Given a work unit with an ast-research attachment requirement and FSPEC_CAPTURE_MODE is not set
    // @step When I dispatch update-work-unit-status for it with status "testing" without the required attachment
    let message = ast_research_error_message(false);

    // @step Then the error message says to run `fspec astgrep --help` first
    assert!(message
        .contains("Cannot transition to testing - no AST research performed during discovery."));
    assert!(message.contains("FIRST run: fspec astgrep --help"));
    // @step And the error message says to use `fspec astgrep --pattern <pattern> --lang <language> [--path <path>]`
    assert!(message
        .contains("THEN use: fspec astgrep --pattern <pattern> --lang <language> [--path <path>]"));
    // @step And the error message does not mention `fspec research --tool=ast`
    assert!(!message.contains("Use the AstGrep tool"));
    assert!(!message.contains("fspec research --tool=ast"));
}

#[test]
fn ast_research_message_capture_variant_directs_to_astgrep() {
    // @step Given a work unit with an ast-research attachment requirement and FSPEC_CAPTURE_MODE is set to "1"
    // @step When I dispatch update-work-unit-status for it with status "testing" without the required attachment
    let message = ast_research_error_message(true);

    // @step Then the error message says to use the AstGrep tool to analyze relevant code
    assert!(message
        .contains("Cannot transition to testing - no AST research performed during discovery."));
    assert!(message.contains("Use the AstGrep tool to analyze relevant code in the codebase."));
    // @step And the error message does not mention `fspec astgrep`
    assert!(!message.contains("fspec astgrep"));
}

#[test]
fn capture_mode_defaults_to_false_for_cli() {
    // @step Given FSPEC_CAPTURE_MODE is not set to "1" (CLI default)
    // @step Then capture mode is false, matching the TS CLI default
    if std::env::var("FSPEC_CAPTURE_MODE")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        // Environment explicitly opts in; nothing to assert in that case.
        return;
    }
    assert!(!is_in_capture_mode());
}

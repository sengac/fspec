@querying
@done
@cli
@astgrep
@CLI-015
Feature: Specifying-to-testing error directs per mode

  """
  The `specifying→testing` AST-research gate error is mode-aware (CLI-015).
  In CLI mode it points at `fspec astgrep --help` then `fspec astgrep
  --pattern ... --lang ... [--path ...]`; in harness mode (FSPEC_CAPTURE_MODE=1)
  it points at the native AstGrep tool. Rendered by
  ast_research_error_message(capture) in
  rust/fspec-core/src/commands/update_work_unit_status.rs.
  """

  Background: User Story
    As a user transitioning a work unit from specifying to testing
    I want the AST-research error to name a tool that exists in my mode
    So that the fix-it steps actually work

  Scenario: specifying-to-testing error directs CLI users to fspec astgrep
    Given a work unit with an ast-research attachment requirement and FSPEC_CAPTURE_MODE is not set
    When I dispatch update-work-unit-status for it with status "testing" without the required attachment
    Then the error message says to run `fspec astgrep --help` first
    And the error message says to use `fspec astgrep --pattern <pattern> --lang <language> [--path <path>]`
    And the error message does not mention `fspec research --tool=ast`

  Scenario: specifying-to-testing error directs harness agents to the AstGrep tool
    Given a work unit with an ast-research attachment requirement and FSPEC_CAPTURE_MODE is set to "1"
    When I dispatch update-work-unit-status for it with status "testing" without the required attachment
    Then the error message says to use the AstGrep tool to analyze relevant code
    And the error message does not mention `fspec astgrep`

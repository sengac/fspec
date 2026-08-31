@querying
@done
@cli
@astgrep
@CLI-015
Feature: Specifying reminder names the AST search tool per mode

  """
  The SPECIFYING status-change reminder is mode-aware (CLI-015): in harness
  mode (FSPEC_CAPTURE_MODE=1) it names the native AstGrep tool; in CLI mode
  it names the `fspec astgrep` subcommand. Rendered via
  ast_research_block(in_capture) in
  rust/fspec-core/src/commands/update_work_unit_status/reminders.rs.
  """

  Background: User Story
    As an agent or shell user entering the specifying phase
    I want the reminder to name an AST search tool that exists in my mode
    So that the guidance never points at a dead-end command

  Scenario: CLI-mode specifying reminder points at the fspec astgrep command
    Given a work unit exists in specifying-relevant state and FSPEC_CAPTURE_MODE is not set
    When I dispatch update-work-unit-status for it with status "specifying"
    Then the specifying reminder in the response references `fspec astgrep`
    And the specifying reminder does not reference `fspec research --tool=ast`

  Scenario: harness-mode specifying reminder points at the AstGrep tool
    Given a work unit exists in specifying-relevant state and FSPEC_CAPTURE_MODE is set to "1"
    When I dispatch update-work-unit-status for it with status "specifying"
    Then the specifying reminder in the response references the AstGrep tool
    And the specifying reminder does not reference `fspec astgrep`

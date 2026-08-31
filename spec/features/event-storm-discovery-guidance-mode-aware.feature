@done
@querying
@cli
@astgrep
@CLI-015
Feature: Event-storm discovery guidance is mode-aware

  """
  The discover-event-storm guidance's Research-First Workflow block is
  rendered per mode (CLI-015): harness mode (FSPEC_CAPTURE_MODE=1) names the
  native AstGrep tool; CLI mode names the `fspec astgrep` subcommand.
  Rendered by event_storm_guidance() in
  rust/fspec-core/src/commands/discover_event_storm.rs.
  """

  Background: User Story
    As a user running event-storm discovery
    I want the research-first guidance to name an AST search tool that exists in my mode
    So that the guidance never points at a dead-end command

  Scenario: event-storm discovery guidance is mode-aware
    Given a work unit is in specifying status and FSPEC_CAPTURE_MODE is not set
    When I dispatch discover-event-storm for it
    Then the emitted guidance references `fspec astgrep` for the research-first step
    And when FSPEC_CAPTURE_MODE is set to "1" I dispatch discover-event-storm for it
    Then the emitted guidance references the `AstGrep` tool for the research-first step

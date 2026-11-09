@done
@BUG-495
@cli
@feature-management
@rust
Feature: delete-scenario help lists only accepted options
  """
  'fspec delete-scenario --help' documented a --force option ("Skip
  confirmation"), copied from the TypeScript help config, but the command has
  never asked for confirmation and its parser rejects --force as an unknown
  argument (seen during BUG-477). The option is removed from the help config
  (rust/fspec-core/src/help/configs/delete_scenario.rs) and its fixture, so
  the help and the parser agree.
  Tests: rust/fspec/tests/bug495_delete_scenario_help_lists_accepted_options.rs.
  """

  Background: User Story
    As a maintainer reading fspec help
    I want to see only the options delete-scenario really accepts
    So that a documented option never fails as an unknown argument

  Scenario: The help does not offer a force option
    When I run fspec delete-scenario --help
    Then the output does not mention --force

  Scenario: The force option stays an unknown argument
    Given a feature file with scenario "Old"
    When I run fspec delete-scenario with the feature, "Old" and --force
    Then the command fails reporting --force as an unknown argument
    And scenario "Old" is still in the feature file

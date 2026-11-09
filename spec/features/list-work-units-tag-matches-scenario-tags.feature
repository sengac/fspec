@done
@CLI-018
@rust
@querying
@cli
Feature: list-work-units --tag matches scenario tags too
  """
  Follow-up to CLI-017 (product owner, 2026-11-08, via hitl): fspec
  list-work-units --tag @x selects a card when one of its feature files
  carries @x on the Feature line or on any scenario's tag line (Scenario
  Outline and scenarios inside a Rule included). A card's feature files are
  unchanged from CLI-017: those whose Feature-line tags hold the card's @ID.
  Tags are matched exactly and read from the Gherkin parse, so @cli written
  in a description, docstring, step or comment is not a tag. A feature file
  that cannot be parsed while --tag is given is an error naming the file.
  Core: rust/fspec-core/src/commands/list_work_units/filter.rs. Tests:
  rust/fspec-core/tests/cli018_tag_matches_scenario_tags.rs.
  """

  Background: User Story
    As a AI agent or developer querying the board
    I want to select cards with --tag by a tag on the Feature line or on any scenario of their feature files
    So that a card whose feature tags a scenario @x is found by --tag @x, not only one whose Feature line does

  Scenario: A tag on one scenario of a card's feature file selects the card
    Given spec/features/a.feature carries @CLI-001 on its Feature line and @cli only on one scenario
    When I list work units with "--tag @cli --format ids"
    Then the output is the ids CLI-001

  Scenario: A tag that is only text in the feature file does not select the card
    Given spec/features/b.feature carries @CLI-002 and @tui on its Feature line and @cli only in its description and in a step
    When I list work units with "--tag @cli --format ids"
    Then the output is no ids

  Scenario: Tags match exactly
    Given spec/features/c.feature carries @CLI-003 on its Feature line and @cli-tools on a scenario
    When I list work units with "--tag @cli --format ids"
    Then the output is no ids

  Scenario: A tag on a Scenario Outline inside a Rule selects the card
    Given spec/features/d.feature carries @CLI-004 on its Feature line and @cli on a Scenario Outline inside a Rule
    When I list work units with "--tag @cli --format ids"
    Then the output is the ids CLI-004

  Scenario: A feature file that does not parse is an error naming it
    Given spec/features/e.feature carries @CLI-005 on its Feature line and does not parse as Gherkin
    When I list work units with "--tag @cli --format ids"
    Then the query fails with an error naming spec/features/e.feature

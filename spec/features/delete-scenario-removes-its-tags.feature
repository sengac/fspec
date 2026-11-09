@done
@BUG-443
@cli
@validation
Feature: Deleting a scenario removes its tags
  """
  delete-scenario (rust/fspec-core/src/commands/delete_scenario.rs) removed
  the span from the Scenario line through its trailing blank lines, leaving
  the scenario's own tag lines above it. Deleting a tagged last scenario
  then produced invalid Gherkin and the command refused. The span now also
  covers the tag lines directly above the scenario; the next scenario's
  tags and the feature's tags are untouched. Tests:
  rust/fspec-core/tests/bug443_delete_scenario_removes_tags.rs.
  """

  Background: User Story
    As a developer deleting a tagged scenario
    I want its tags to go with it
    So that the feature file stays valid Gherkin

  Scenario: Deleting a tagged last scenario removes its tag line
    Given a feature "login" whose last scenario "Old" is tagged "@legacy"
    When the user deletes the scenario "Old" from "login"
    Then the deletion succeeds
    And "login" no longer contains "@legacy"

  Scenario: Deleting a scenario keeps the next scenario's tags
    Given a feature "login" with scenario "Old" followed by scenario "Keep" tagged "@smoke"
    When the user deletes the scenario "Old" from "login"
    Then the deletion succeeds
    And "login" still tags "Keep" with "@smoke"

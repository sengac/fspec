@done
@BUG-444
@cli
@coverage
@validation
Feature: Scenario deletion reports coverage file errors
  """
  delete-scenario and delete-scenarios updated the .feature.coverage sidecar
  best-effort: read, parse and write failures were discarded, so a scenario
  could be deleted while its coverage entry silently stayed. The shared
  coverage_prune helper (rust/fspec-core/src/commands/coverage_prune.rs)
  prepares the pruned coverage before the feature is written: a missing
  coverage file is still a no-op, any other failure fails the command
  naming the file and nothing is written. Tests:
  rust/fspec-core/tests/bug444_scenario_deletion_coverage_errors.rs.
  """

  Background: User Story
    As a developer deleting scenarios
    I want a broken coverage file reported instead of skipped
    So that the feature and its coverage never silently disagree

  Scenario: A malformed coverage file fails delete-scenario and leaves the feature unchanged
    Given a feature "login" with scenario "Old" whose coverage file is not valid JSON
    When the user deletes the scenario "Old" from "login"
    Then the deletion fails with an error naming "login.feature.coverage"
    And "login" still contains the scenario "Old"

  Scenario: A malformed coverage file fails delete-scenarios and leaves the feature unchanged
    Given a feature "login" with scenario "Old" tagged "@legacy" whose coverage file is not valid JSON
    When the user deletes the scenarios tagged "@legacy"
    Then the deletion fails with an error naming "login.feature.coverage"
    And "login" still contains the scenario "Old"

@BUG-582
@done
@BUG-374
@cli
@coverage
@validation
Feature: Coverage stats after scenario deletion
  """
  delete-scenario and delete-scenarios
  (rust/fspec-core/src/commands/delete_scenario.rs, delete_scenarios.rs)
  recomputed only totalScenarios, coveredScenarios and coveragePercent, so
  stats.testFiles and stats.implFiles kept files no remaining scenario maps
  (seen on exec-stdin-prompt.feature.coverage in TUI-116). Both now
  recompute every stat with the shared link-coverage update_stats. Tests:
  rust/fspec-core/tests/bug374_coverage_stats_after_deletion.rs.
  """

  Background: User Story
    As a developer removing scenarios from a feature
    I want the coverage stats to describe only what remains
    So that coverage reports never list files no scenario uses

  Scenario: Deleting a scenario drops its test and implementation files from the stats
    Given a feature "login" whose scenario "Old" alone maps "tests/old.rs" and "src/old.rs"
    When the user deletes the scenario "Old" from "login"
    Then the coverage stats no longer list "tests/old.rs" or "src/old.rs"

  Scenario: Deleting scenarios by tag drops their files from the stats
    Given a feature "login" whose scenario "Old" alone maps "tests/old.rs" and "src/old.rs" and is tagged "@legacy"
    When the user deletes the scenarios tagged "@legacy"
    Then the coverage stats no longer list "tests/old.rs" or "src/old.rs"

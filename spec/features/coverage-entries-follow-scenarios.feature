@done
@BUG-346
@cli
@coverage
@validation
Feature: Coverage entries follow added scenarios
  """
  add-scenario (rust/fspec-core/src/commands/add_scenario.rs) wrote the new
  scenario to the .feature but not to its .feature.coverage, so link-coverage
  rejected the scenario until a hand-added entry existed. link-coverage's
  update_stats (link_coverage/stats.rs) recomputed coveredScenarios but kept
  the stored totalScenarios, producing e.g. 7 covered of 5 (140%).
  add-scenario now appends an empty entry to an existing coverage file, and
  update_stats (shared with add-scenario) derives totalScenarios and
  coveragePercent from the scenarios list. Tests:
  rust/fspec-core/tests/bug346_coverage_entries_follow_scenarios.rs.
  """

  Background: User Story
    As a developer adding scenarios to an existing feature
    I want its coverage file to track the new scenario immediately
    So that I can link tests to it and coverage totals stay truthful

  Scenario: Adding a scenario adds an empty coverage entry
    Given a feature "login" with one scenario and a coverage file
    When the user adds the scenario "Reset password" to "login"
    Then the coverage file lists "Reset password" with no test mappings
    And the coverage stats report 2 total scenarios

  Scenario: Linking coverage recomputes the total from the scenario list
    Given a coverage file for "login" listing 2 scenarios whose stats claim 1 total scenario
    When the user links a test to one of its scenarios
    Then the coverage stats report 2 total scenarios and 50 percent coverage

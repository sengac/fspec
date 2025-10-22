@validation
@ai-guidance
@system-reminder
@error-handling
@cli
@UX-001
Feature: System-reminder for missing scenarios in link-coverage
  """
  System-reminder follows same pattern as other AI guidance features (wrapped in <system-reminder> tags)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When link-coverage fails with 'Scenario not found', check if scenario exists in feature file
  #   2. If scenario exists in feature file but not in coverage file, emit system-reminder suggesting generate-coverage
  #   3. System-reminder must be wrapped in <system-reminder> tags (visible to AI, invisible to user)
  #   4. If scenario doesn't exist in feature file either, show normal error without system-reminder
  #
  # EXAMPLES:
  #   1. AI adds new scenario to feature file, runs link-coverage before generate-coverage → error with system-reminder: 'Run generate-coverage first'
  #   2. AI tries to link non-existent scenario (typo in scenario name) → error without system-reminder, just shows available scenarios
  #   3. Coverage file missing entirely → system-reminder suggests running generate-coverage to create it
  #
  # ========================================
  Background: User Story
    As a AI agent (Claude) using fspec
    I want to receive helpful guidance when link-coverage fails due to missing scenarios
    So that I can quickly fix the issue without trial and error

  Scenario: Scenario exists in feature file but not in coverage file
    Given I have a feature file with a scenario "Login with valid credentials"
    Given the coverage file exists but does not contain that scenario
    When I run "fspec link-coverage" for that scenario
    Then the command should fail with "Scenario not found" error
    Then the output should contain a system-reminder wrapped in <system-reminder> tags
    Then the system-reminder should suggest running "fspec generate-coverage" first

  Scenario: Scenario doesn't exist in feature file (typo)
    Given I have a feature file with scenarios
    Given the coverage file exists
    When I run "fspec link-coverage" with a scenario name that doesn't exist in the feature file
    Then the command should fail with "Scenario not found" error
    Then the output should list available scenarios
    Then the output should NOT contain a system-reminder

  Scenario: Coverage file missing entirely
    Given I have a feature file with a scenario "Login with valid credentials"
    Given the coverage file does not exist
    When I run "fspec link-coverage" for that scenario
    Then the command should fail with an error
    Then the output should contain a system-reminder suggesting "fspec generate-coverage"

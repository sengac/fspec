@done
@workflow-automation
@hooks
@HOOK-006
Feature: System reminder formatting for AI

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # USER STORY:
  #   As a fspec system
  #   I want to format blocking hook stderr as system-reminders for AI agents
  #   So that AI agents receive actionable feedback when hooks fail
  #
  # BUSINESS RULES:
  #   1. Blocking hook stderr is wrapped in <system-reminder> tags
  #   2. Non-blocking hook stderr is displayed as-is (no wrapping)
  #   3. System-reminder content includes hook name, exit code, and stderr output
  #   4. Empty stderr produces no system-reminder (only if stderr has content)
  #   5. System-reminder is appended to command output (not replacing it)
  #
  # EXAMPLES:
  #   1. Blocking hook fails with stderr 'Invalid config' - wrapped in system-reminder
  #   2. Non-blocking hook fails with stderr - displayed as-is, no wrapping
  #   3. Blocking hook succeeds - no system-reminder generated
  #   4. Blocking hook fails with empty stderr - no system-reminder
  #   5. System-reminder includes: hook name 'validate', exit code 1, stderr content
  #
  # ========================================
  Background: User Story
    As a fspec system
    I want to format blocking hook stderr as system-reminders for AI agents
    So that AI agents receive actionable feedback when hooks fail

  Scenario: Blocking hook fails with stderr wrapped in system-reminder
    Given I have a blocking hook that fails with exit code 1
    And the hook outputs "Invalid config" to stderr
    When I format the hook output for display
    Then the stderr should be wrapped in system-reminder tags
    And the system-reminder should include the hook name
    And the system-reminder should include the exit code
    And the system-reminder should include the stderr content

  Scenario: Non-blocking hook stderr displayed as-is
    Given I have a non-blocking hook that fails with exit code 1
    And the hook outputs "Warning: deprecated API" to stderr
    When I format the hook output for display
    Then the stderr should be displayed as-is
    And the stderr should not be wrapped in system-reminder tags

  Scenario: Blocking hook succeeds with no system-reminder
    Given I have a blocking hook that succeeds with exit code 0
    And the hook outputs "All checks passed" to stdout
    When I format the hook output for display
    Then no system-reminder should be generated
    And only stdout should be displayed

  Scenario: Blocking hook fails with empty stderr produces no system-reminder
    Given I have a blocking hook that fails with exit code 1
    And the hook produces no stderr output
    When I format the hook output for display
    Then no system-reminder should be generated

  Scenario: System-reminder includes hook metadata
    Given I have a blocking hook named "validate" that fails
    And the hook exits with code 1
    And the hook outputs "Validation failed: missing field 'name'" to stderr
    When I format the hook output for display
    Then the system-reminder should contain "Hook: validate"
    And the system-reminder should contain "Exit code: 1"
    And the system-reminder should contain "Validation failed: missing field 'name'"

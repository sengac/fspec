@cli
@git-ops
@workflow-automation
@critical
@init
@git
@checkpoints
@config
@platform-agnostic
@INIT-014
Feature: Replace hardcoded npm test in git-checkpoint conflict resolution with configured test command

  """
  The detectConflicts function in git-checkpoint.ts must load fspec config using loadConfig(cwd) from src/utils/config.ts. This allows access to config.tools.test.command which contains the configured test command. The function must be modified to accept cwd parameter (already present) and replace the hardcoded 'npm test' string with the configured value. If config loading fails or test command is not set, fallback to generic language 'your configured test command'.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The detectConflicts function must load fspec config to get the configured test command
  #   2. System-reminder text must use the configured test command instead of hardcoded 'npm test'
  #   3. If config loading fails or test command is not configured, fallback to generic language 'your configured test command'
  #
  # EXAMPLES:
  #   1. Python project with 'pytest' configured - system-reminder shows 'Run: pytest' not 'npm test'
  #   2. Rust project with 'cargo test' configured - system-reminder shows 'Run: cargo test'
  #   3. Go project with 'go test ./...' configured - system-reminder shows 'Run: go test ./...'
  #   4. Project without fspec-config.json - system-reminder shows 'Run: your configured test command' as fallback
  #   5. JavaScript project with 'npm test' configured - system-reminder correctly shows 'Run: npm test' (proving config loading works)
  #
  # ========================================

  Background: User Story
    As a AI agent
    I want to see platform-agnostic test commands in git conflict resolution guidance
    So that I can resolve conflicts and run the correct test command for my project (pytest, cargo test, go test, npm test, etc.)

  Scenario: Python project shows pytest in conflict resolution
    Given I have a Python project with "pytest" configured as test command
    And git checkpoint restoration causes merge conflicts
    When the detectConflicts function generates the system-reminder
    Then the system-reminder should contain "Run: pytest"
    And the system-reminder should NOT contain "Run: npm test"

  Scenario: Rust project shows cargo test in conflict resolution
    Given I have a Rust project with "cargo test" configured as test command
    And git checkpoint restoration causes merge conflicts
    When the detectConflicts function generates the system-reminder
    Then the system-reminder should contain "Run: cargo test"
    And the system-reminder should NOT contain "Run: npm test"

  Scenario: Go project shows go test in conflict resolution
    Given I have a Go project with "go test ./..." configured as test command
    And git checkpoint restoration causes merge conflicts
    When the detectConflicts function generates the system-reminder
    Then the system-reminder should contain "Run: go test ./..."
    And the system-reminder should NOT contain "Run: npm test"

  Scenario: Project without config shows generic fallback
    Given I have a project without spec/fspec-config.json
    And git checkpoint restoration causes merge conflicts
    When the detectConflicts function generates the system-reminder
    Then the system-reminder should contain "Run: your configured test command"
    And the system-reminder should NOT contain "Run: npm test"

  Scenario: JavaScript project shows npm test when configured
    Given I have a JavaScript project with "npm test" configured as test command
    And git checkpoint restoration causes merge conflicts
    When the detectConflicts function generates the system-reminder
    Then the system-reminder should contain "Run: npm test"

@help-system
@refactoring
@cli
@high
Feature: Replace generic create-work-unit references with type-specific commands

  """
  This refactoring requires manual context analysis for each occurrence of 'create-work-unit'. No automated find-replace allowed since context determines the correct command. Files affected: spec/CLAUDE.md, help text files (src/commands/*-help.ts), README.md, and potentially other documentation files. Each instance must be evaluated to determine if it refers to a feature/refactoring (create-story), bug fix (create-bug), or operational task (create-task).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each reference to 'create-work-unit' must be analyzed for context to determine if it should be 'create-story', 'create-bug', or 'create-task'
  #   2. Documentation examples about new features or refactoring should use 'create-story'
  #   3. Documentation examples about fixing bugs should use 'create-bug'
  #   4. Documentation examples about operational work (setup, configuration, cleanup) should use 'create-task'
  #   5. All changes must preserve the intent and accuracy of the original documentation
  #
  # EXAMPLES:
  #   1. In spec/CLAUDE.md, example showing 'fspec create-work-unit AUTH "User login feature"' should become 'fspec create-story AUTH "User login feature"'
  #   2. In help text, example showing 'fspec create-work-unit DASH "Dashboard"' should become 'fspec create-story DASH "Dashboard"'
  #   3. References to creating work for 'Add ESLint' should use 'fspec create-task' since it's operational setup
  #   4. References to creating work for 'Fix validation bug' should use 'fspec create-bug'
  #
  # ========================================

  Background: User Story
    As a developer or AI agent using fspec
    I want to see contextually appropriate command examples in documentation
    So that I understand which command to use for creating stories, bugs, or tasks

  Scenario: Replace create-work-unit with create-story for feature examples
    Given I have documentation with "fspec create-work-unit AUTH \"User login feature\""
    When I analyze the context and determine it refers to a new feature
    Then I should replace it with "fspec create-story AUTH \"User login feature\""
    And the documentation intent should remain accurate

  Scenario: Replace create-work-unit with create-task for operational examples
    Given I have documentation with "fspec create-work-unit" for "Add ESLint"
    When I analyze the context and determine it refers to operational setup
    Then I should replace it with "fspec create-task"
    And the documentation intent should remain accurate

  Scenario: Replace create-work-unit with create-bug for bug fix examples
    Given I have documentation with "fspec create-work-unit" for "Fix validation bug"
    When I analyze the context and determine it refers to fixing a bug
    Then I should replace it with "fspec create-bug"
    And the documentation intent should remain accurate

  Scenario: Update help text to use create-story for dashboard example
    Given I have help text showing "fspec create-work-unit DASH \"Dashboard\""
    When I analyze the context and determine it refers to a new feature
    Then I should replace it with "fspec create-story DASH \"Dashboard\""
    And the help text should accurately guide users

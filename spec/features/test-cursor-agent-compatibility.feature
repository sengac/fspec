@medium
@cli
@agent-support
@integration
@AGENT-001
Feature: Test Cursor agent compatibility
  """
  Tests multi-agent support system for Cursor AI editor. Cursor uses .cursor/commands/ directory for custom commands. Initialization creates agent-specific files: .cursor/commands/fspec.md (command integration), spec/CURSOR.md (agent instructions). Validates file creation, content structure, and activation message display. Uses vitest for testing, fs/promises for file operations.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. fspec init --agent=cursor must create .cursor/commands/fspec.md file with fspec command integration
  #   2. fspec init --agent=cursor must create spec/CURSOR.md with agent-specific instructions and ACDD workflow
  #   3. Cursor activation instructions must be displayed after successful initialization
  #   4. All core fspec commands must work in Cursor environment (create-story, add-scenario, generate-scenarios, etc.)
  #
  # EXAMPLES:
  #   1. Run 'fspec init --agent=cursor' in empty project → creates .cursor/commands/fspec.md and spec/CURSOR.md
  #   2. After init, activation message shows: 'Cursor: open command palette and select fspec command'
  #   3. Created .cursor/commands/fspec.md contains fspec CLI command invocations and ACDD workflow instructions
  #   4. Created spec/CURSOR.md contains Cursor-specific ACDD guidelines and Example Mapping workflow
  #
  # ========================================
  Background: User Story
    As a developer using Cursor AI agent
    I want to initialize fspec with Cursor-specific configuration
    So that I can use fspec's ACDD workflow within Cursor environment

  Scenario: Initialize fspec with Cursor agent
    Given I am in an empty project directory
    When I run "fspec init --agent=cursor"
    Then a file ".cursor/commands/fspec.md" should be created
    And a file "spec/CURSOR.md" should be created
    And both files should exist in the project

  Scenario: Display Cursor activation instructions after init
    Given I am in an empty project directory
    When I run "fspec init --agent=cursor"
    Then the output should contain "Cursor: open command palette and select fspec command"
    And activation instructions should be displayed

  Scenario: Created command file contains fspec CLI integration
    Given I have run "fspec init --agent=cursor"
    When I read the file ".cursor/commands/fspec.md"
    Then it should contain "fspec" CLI command references
    And it should contain ACDD workflow instructions
    And it should describe how to use fspec commands in Cursor

  Scenario: Created spec file contains Cursor-specific guidelines
    Given I have run "fspec init --agent=cursor"
    When I read the file "spec/CURSOR.md"
    Then it should contain "Cursor" agent-specific instructions
    And it should contain ACDD workflow guidelines
    And it should contain Example Mapping workflow instructions

@done
@cli
@discovery
@help-text
@documentation
@FOUND-006
Feature: Update Documentation and Help
  """
  Architecture notes:
  - This is a documentation-only work unit (no code implementation)
  - Updates AGENTS.md with discover-foundation workflow section
  - Creates help config for discover-foundation command
  - Adds discovery guide documentation explaining code analysis patterns
  - Validates documentation quality through test scenarios
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AGENTS.md must document discover-foundation workflow with examples
  #   2. Help system must have comprehensive --help output for discover-foundation command
  #   3. Discovery guide must explain code analysis patterns (CLI tool, web app, library)
  #   4. Documentation must show integration with Example Mapping workflow
  #
  # EXAMPLES:
  #   1. AGENTS.md shows: 'Run discover-foundation to analyze codebase and generate foundation.json'
  #   2. Help output includes WHEN TO USE, WORKFLOW, and EXAMPLES sections
  #   3. Discovery guide explains CLI tool pattern: bin field in package.json, commander usage
  #
  # ========================================
  Background: User Story
    As a developer using fspec for foundation document discovery
    I want to have comprehensive documentation and help for discovery commands
    So that I can effectively use discover-foundation, questionnaire, and code analysis features

  Scenario: AGENTS.md documents discover-foundation workflow
    Given I run "fspec init" for the claude agent
    When I search spec/AGENTS.md for the "Bootstrapping Foundation for New Projects" section
    Then I should find workflow documentation with command examples
    And documentation should show how to run discover-foundation command
    And documentation should describe the draft, field-by-field and finalize workflow and point to fspec discover-foundation --help

  Scenario: Help system provides comprehensive command help
    Given I run 'fspec discover-foundation --help'
    When the help output is displayed
    Then it should include "WHEN TO USE" section
    And it should include "EXAMPLES" section with concrete usage
    And examples should show typical discovery workflow

  Scenario: Discovery guide explains code analysis patterns
    Given I run 'fspec discover-foundation --help'
    When I review its COMMON PATTERNS section
    Then I should find the CLI tools pattern: CLI framework usage and command definitions
    And I should find the web apps pattern: routes and UI components
    And I should find the libraries pattern: exports and API
    And each pattern should name the persona to identify, not how to implement it

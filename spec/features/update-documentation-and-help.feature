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
  - Updates CLAUDE.md with discover-foundation workflow section
  - Creates help config for discover-foundation command
  - Adds discovery guide documentation explaining code analysis patterns
  - Validates documentation quality through test scenarios
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CLAUDE.md must document discover-foundation workflow with examples
  #   2. Help system must have comprehensive --help output for discover-foundation command
  #   3. Discovery guide must explain code analysis patterns (CLI tool, web app, library)
  #   4. Documentation must show integration with Example Mapping workflow
  #
  # EXAMPLES:
  #   1. CLAUDE.md shows: 'Run discover-foundation to analyze codebase and generate foundation.json'
  #   2. Help output includes WHEN TO USE, WORKFLOW, and EXAMPLES sections
  #   3. Discovery guide explains CLI tool pattern: bin field in package.json, commander usage
  #
  # ========================================
  Background: User Story
    As a developer using fspec for foundation document discovery
    I want to have comprehensive documentation and help for discovery commands
    So that I can effectively use discover-foundation, questionnaire, and code analysis features

  Scenario: CLAUDE.md documents discover-foundation workflow
    Given I open CLAUDE.md file
    When I search for "discover-foundation" section
    Then I should find workflow documentation with command examples
    And documentation should show how to run discover-foundation command
    And documentation should explain integration with questionnaire

  Scenario: Help system provides comprehensive command help
    Given I run 'fspec discover-foundation --help'
    When the help output is displayed
    Then it should include "WHEN TO USE" section
    And it should include "WORKFLOW" section explaining steps
    And it should include "EXAMPLES" section with concrete usage
    And examples should show typical discovery workflow

  Scenario: Discovery guide explains code analysis patterns
    Given I open discovery guidance documentation
    When I review code analysis patterns
    Then I should find CLI tool detection pattern with bin field example
    And I should find web app detection pattern with routes example
    And I should find library detection pattern with exports example
    And each pattern should explain WHAT to infer not HOW to implement

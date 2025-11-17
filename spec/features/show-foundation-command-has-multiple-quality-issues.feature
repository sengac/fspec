@cli
@foundation-management
@querying
@high
@bug-fix
@BUG-081
Feature: show-foundation command has multiple quality issues

  """
  Fixes option name mismatch between registration (--section) and handler (options.field). Adds positional argument support for field parameter. Maintains backward compatibility with FIELD_MAP for mapped aliases and JSON path notation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The --section parameter must correctly filter to show only the specified field
  #   2. Positional argument syntax 'fspec show-foundation <field>' must work as documented in help text
  #   3. Both --section flag and positional argument must accept field names from FIELD_MAP or JSON paths
  #   4. When no field specified, show entire foundation in formatted text
  #   5. Auto-creation of foundation.json with template data is expected behavior when file is missing
  #
  # EXAMPLES:
  #   1. Run 'fspec show-foundation --section projectOverview' and see only the project overview content, not the entire foundation
  #   2. Run 'fspec show-foundation projectOverview' (positional) and get the same result as using --section flag
  #   3. Run 'fspec show-foundation solutionSpace.overview' using JSON path and retrieve the nested field
  #   4. Run 'fspec show-foundation' with no arguments and see the entire foundation formatted as text with sections
  #   5. Run 'fspec show-foundation --section nonExistentField' and receive error message 'Field not found'
  #
  # ========================================

  Background: User Story
    As a developer using show-foundation command
    I want to view foundation data with correct parameter handling
    So that I can access foundation content using documented syntax without confusion

  Scenario: Filter foundation using --section flag
    Given I have a foundation.json with projectOverview field
    When I run `fspec show-foundation --section projectOverview`
    Then the command should exit with code 0
    And the output should display only the project overview content
    And the output should not contain other foundation fields


  Scenario: Filter foundation using positional argument
    Given I have a foundation.json with projectOverview field
    When I run `fspec show-foundation projectOverview`
    Then the command should exit with code 0
    And the output should display only the project overview content
    And the result should match --section flag output


  Scenario: Filter foundation using JSON path notation
    Given I have a foundation.json with nested solutionSpace.overview field
    When I run `fspec show-foundation solutionSpace.overview`
    Then the command should exit with code 0
    And the output should display only the solution space overview content


  Scenario: Display entire foundation without field filter
    Given I have a complete foundation.json file
    When I run `fspec show-foundation`
    Then the command should exit with code 0
    And the output should display formatted text with all sections
    And the output should contain PROJECT section
    And the output should contain PROBLEM SPACE section
    And the output should contain SOLUTION SPACE section


  Scenario: Handle non-existent field error
    Given I have a foundation.json file
    When I run `fspec show-foundation --section nonExistentField`
    Then the command should exit with code 1
    And the output should show "Field 'nonExistentField' not found"


@phase6
@cli
@foundation-management
@querying
@low
@unit-test
Feature: Display Foundation Documentation
  """
  Architecture notes:
  - Reads spec/foundation.json (source of truth) and displays content
  - Can show entire foundation or specific field
  - Supports multiple output formats (text, markdown, json)
  - Can write output to a file
  - Uses Foundation type structure for data access
  - Provides structured data for programmatic access

  Critical implementation requirements:
  - MUST load spec/foundation.json
  - MUST support showing entire foundation data
  - MUST support showing specific field by path (e.g., "projectOverview")
  - MUST support output formats: text, markdown, json
  - MUST support --output flag to write to file
  - MUST handle missing foundation.json gracefully
  - MUST handle missing field gracefully
  - Exit code 0 for success, 1 for errors

  Workflow:
  1. Load spec/foundation.json
  2. Parse Foundation data structure
  3. Extract specific field if requested
  4. Format output based on --format flag
  5. Display or write to file
  6. Handle errors with clear messages

  References:
  - foundation.schema.json: src/schemas/foundation.schema.json
  - Foundation type: src/types/foundation.ts
  """

  Background: User Story
    As a developer reviewing project documentation
    I want to display foundation content from JSON
    So that I can view and extract foundation documentation programmatically

  Scenario: Display entire foundation in JSON format
    Given I have a foundation.json with complete project data
    When I run `fspec show-foundation --format json`
    Then the command should exit with code 0
    And the output should be valid JSON
    And the JSON should contain all foundation fields

  Scenario: Display specific field
    Given I have a foundation.json with projectOverview field
    When I run `fspec show-foundation --field projectOverview`
    Then the command should exit with code 0
    And the output should display only that field content
    And other fields should not be displayed

  Scenario: Display in text format (default)
    Given I have a foundation.json
    When I run `fspec show-foundation`
    Then the command should exit with code 0
    And the output should display foundation content as readable text
    And project name and description should be shown

  Scenario: Write JSON output to file
    Given I have a foundation.json
    When I run `fspec show-foundation --format json --output foundation-copy.json`
    Then the command should exit with code 0
    And a file "foundation-copy.json" should be created
    And it should contain valid JSON with foundation data

  Scenario: Handle missing foundation.json
    Given I have no foundation.json file
    When I run `fspec show-foundation`
    Then the command should exit with code 1
    And the output should show "foundation.json not found"

  Scenario: Handle missing field
    Given I have a foundation.json
    When I run `fspec show-foundation --field nonExistentField`
    Then the command should exit with code 1
    And the output should show "Field 'nonExistentField' not found"

  Scenario: JSON-backed workflow - read from source of truth
    Given I have a valid foundation.json file
    When I run `fspec show-foundation --format json`
    Then the command should load data from spec/foundation.json
    And the output should be valid JSON matching the Foundation schema
    And all top-level fields should be present
    And the command should exit with code 0

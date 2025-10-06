@phase6
@cli
@foundation-management
@querying
@low
@unit-test
Feature: Display Foundation Documentation
  """
  Architecture notes:
  - Displays content from FOUNDATION.md file
  - Can show entire file or specific section
  - Supports multiple output formats (text, markdown, json)
  - Can write output to a file
  - Uses standard markdown parsing for sections
  - Provides structured data for programmatic access

  Critical implementation requirements:
  - MUST support showing entire FOUNDATION.md
  - MUST support showing specific section by name
  - MUST support output formats: text, markdown, json
  - MUST support --output flag to write to file
  - MUST handle missing FOUNDATION.md gracefully
  - MUST handle missing section gracefully
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer reviewing project documentation
    I want to display FOUNDATION.md content
    So that I can view and extract foundation documentation



  Scenario: Display entire FOUNDATION.md
    Given I have a FOUNDATION.md with multiple sections
    When I run `fspec show-foundation`
    Then the command should exit with code 0
    And the output should display all sections
    And the output should be in text format

  Scenario: Display specific section
    Given I have a FOUNDATION.md with a "What We Are Building" section
    When I run `fspec show-foundation --section "What We Are Building"`
    Then the command should exit with code 0
    And the output should display only that section
    And other sections should not be displayed

  Scenario: Display in markdown format
    Given I have a FOUNDATION.md
    When I run `fspec show-foundation --format markdown`
    Then the command should exit with code 0
    And the output should preserve markdown formatting
    And section headers should use ## syntax

  Scenario: Display in JSON format
    Given I have a FOUNDATION.md with "Why" and "Architecture" sections
    When I run `fspec show-foundation --format json`
    Then the command should exit with code 0
    And the output should be valid JSON
    And the JSON should contain section names as keys
    And the JSON should contain section content as values

  Scenario: Write output to file
    Given I have a FOUNDATION.md
    When I run `fspec show-foundation --output foundation-copy.md`
    Then the command should exit with code 0
    And a file "foundation-copy.md" should be created
    And it should contain the FOUNDATION.md content

  Scenario: Write specific section to file
    Given I have a FOUNDATION.md with a "Why" section
    When I run `fspec show-foundation --section Why --output why.txt`
    Then the command should exit with code 0
    And a file "why.txt" should be created
    And it should contain only the "Why" section content

  Scenario: Handle missing FOUNDATION.md
    Given I have no FOUNDATION.md file
    When I run `fspec show-foundation`
    Then the command should exit with code 1
    And the output should show "FOUNDATION.md not found"

  Scenario: Handle missing section
    Given I have a FOUNDATION.md without a "Missing Section"
    When I run `fspec show-foundation --section "Missing Section"`
    Then the command should exit with code 1
    And the output should show "Section 'Missing Section' not found"

  Scenario: Display section with subsections
    Given I have an "Architecture" section with diagrams (### subsections)
    When I run `fspec show-foundation --section Architecture`
    Then the command should exit with code 0
    And the output should include the main section content
    And the output should include all subsections (### headers)
    And the output should include diagram content

  Scenario: Display preserves formatting
    Given I have a "Features" section with markdown lists
    When I run `fspec show-foundation --section Features`
    Then the command should exit with code 0
    And the output should preserve list formatting
    And the output should preserve indentation

  Scenario: JSON output includes all sections
    Given I have FOUNDATION.md with 5 sections
    When I run `fspec show-foundation --format json`
    Then the command should exit with code 0
    And the JSON should have 5 top-level keys
    And each key should correspond to a section name

  Scenario: Display section names only
    Given I have a FOUNDATION.md with multiple sections
    When I run `fspec show-foundation --list-sections`
    Then the command should exit with code 0
    And the output should list all section names
    And section content should not be displayed

  Scenario: Display with line numbers
    Given I have a FOUNDATION.md
    When I run `fspec show-foundation --line-numbers`
    Then the command should exit with code 0
    And the output should include line numbers
    And the format should be "N: content"

  Scenario: Handle special characters in section names
    Given I have a section named "What We're Building"
    When I run `fspec show-foundation --section "What We're Building"`
    Then the command should exit with code 0
    And the output should display the section content

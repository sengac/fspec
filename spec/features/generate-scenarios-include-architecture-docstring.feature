@BUG-011
@done
@example-mapping
@bug
@generator
Feature: generate-scenarios missing architecture docstring

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # USER STORY:
  #   As a developer using generate-scenarios
  #   I want to have feature files with architecture docstrings
  #   So that all feature files meet CLAUDE.md requirements and have consistent structure
  #
  # BUSINESS RULES:
  #   1. All feature files MUST have architecture docstrings (CLAUDE.md requirement)
  #   2. generate-scenarios output MUST match create-feature template structure
  #   3. Docstring MUST come before example mapping comments
  #   4. Docstring MUST include TODO placeholders for architecture notes
  #
  # EXAMPLES:
  #   1. User runs generate-scenarios, file contains both docstring and example mapping comments
  #   2. Generated file matches structure: tags → feature → docstring → comments → background
  #   3. Existing tests pass with new docstring addition
  #
  # ========================================
  Background: User Story
    As a developer using generate-scenarios
    I want to have feature files with architecture docstrings
    So that all feature files meet CLAUDE.md requirements and have consistent structure

  Scenario: Generated feature file includes architecture docstring
    Given I have a work unit with example mapping data
    And the work unit has rules and examples
    When I run "fspec generate-scenarios <work-unit-id>"
    Then a feature file should be created
    And the file should contain a docstring with architecture notes placeholder
    And the file should contain example mapping comments
    And the docstring should come before the example mapping comments

  Scenario: Generated file structure matches create-feature template
    Given I run "fspec generate-scenarios" on a work unit
    When I examine the generated feature file
    Then the file structure should be: tags → Feature → docstring → comments → Background
    And the docstring should contain "Architecture notes:"
    And the docstring should contain TODO placeholders
    And the structure should match files created with "fspec create-feature"

  Scenario: Existing tests continue to pass with docstring addition
    Given I modify generate-scenarios to include docstrings
    When I run the existing test suite
    Then all generate-scenarios tests should pass
    And tests should validate docstring presence
    And tests should validate correct ordering (docstring before comments)

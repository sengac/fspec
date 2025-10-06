@phase1
@cli
@formatter
@formatting
@prettier
@cross-platform
@medium
@integration-test
@e2e-test
Feature: Format Feature Files with Prettier
  """
  Architecture notes:
  - Uses Prettier with prettier-plugin-gherkin for formatting
  - Formats all .feature files or specific file
  - Applies consistent indentation (2 spaces)
  - Wraps long lines at 80 characters
  - Preserves doc strings and data tables
  - Writes formatted content back to file

  Critical implementation requirements:
  - MUST use prettier with prettier-plugin-gherkin
  - MUST NOT break valid Gherkin syntax
  - MUST preserve doc strings (""")
  - MUST preserve data tables (|)
  - MUST apply consistent indentation
  - SHOULD show which files were formatted
  - Exit code 0 if successful, 1 if errors

  Prettier configuration (from .prettierrc):
  - parser: gherkin
  - printWidth: 80
  - tabWidth: 2
  - useTabs: false

  Integration points:
  - CLI command: `fspec format [file]`
  - Can be called before validation/commit
  - Integrates with npm run format:spec script
  """

  Background: User Story
    As a developer using AI agents for spec-driven development
    I want to automatically format feature files
    So that all specifications have consistent style



  Scenario: Format a single feature file
    Given I have a feature file "spec/features/login.feature" with inconsistent formatting
    When I run `fspec format spec/features/login.feature`
    Then the command should exit with code 0
    And the file should be formatted with consistent indentation
    And the output should contain "✓ Formatted spec/features/login.feature"

  Scenario: Format all feature files
    Given I have multiple feature files with inconsistent formatting:
      | file                       |
      | spec/features/auth.feature |
      | spec/features/api.feature  |
      | spec/features/data.feature |
    When I run `fspec format`
    Then the command should exit with code 0
    And all 3 files should be formatted
    And the output should contain "✓ Formatted 3 feature files"

  Scenario: Apply consistent indentation
    Given I have a feature file with mixed indentation
    When I run `fspec format spec/features/mixed.feature`
    Then all scenarios should be indented by 2 spaces
    And all steps should be indented by 4 spaces from feature level

  Scenario: Preserve doc strings
    Given I have a feature file with doc strings
    When I run `fspec format spec/features/docs.feature`
    Then the doc strings should be preserved exactly
    And the triple quotes (""") should remain on their own lines

  Scenario: Preserve data tables
    Given I have a feature file with data tables
    When I run `fspec format spec/features/tables.feature`
    Then the data table structure should be preserved
    And columns should be aligned properly

  Scenario: Handle already-formatted file
    Given I have a properly formatted feature file
    When I run `fspec format spec/features/good.feature`
    Then the command should exit with code 0
    And the file content should not change
    And the output should contain "✓ Formatted spec/features/good.feature"

  Scenario: Handle file not found
    Given no file exists at "spec/features/missing.feature"
    When I run `fspec format spec/features/missing.feature`
    Then the command should exit with code 2
    And the output should contain "File not found: spec/features/missing.feature"

  Scenario: Handle empty spec/features directory
    Given I have an empty "spec/features/" directory
    When I run `fspec format`
    Then the command should exit with code 0
    And the output should contain "No feature files found to format"

  Scenario: Show formatted files in output
    Given I have 3 feature files
    When I run `fspec format`
    Then the output should list each formatted file
    And the output should show a summary count

  Scenario: AI agent workflow - create, format, validate
    Given I am an AI agent creating a new specification
    When I run `fspec create-feature "Data Export"`
    And I edit the file to add scenarios
    And I run `fspec format spec/features/data-export.feature`
    Then the file should be properly formatted
    And when I run `fspec validate spec/features/data-export.feature`
    Then the validation should pass

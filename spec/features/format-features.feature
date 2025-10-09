@phase1
@cli
@formatter
@formatting
@ast
@gherkin
@cross-platform
@medium
@integration-test
Feature: Format Feature Files with Custom AST Formatter
  """
  Architecture notes:
  - Uses @cucumber/gherkin parser to generate AST (Abstract Syntax Tree)
  - Custom formatter walks AST and outputs formatted Gherkin
  - Formats all .feature files or specific file
  - Applies consistent indentation (tags: 0, feature: 0, scenarios: 2, steps: 4, tables: 6)
  - Preserves all Gherkin elements exactly from AST
  - Data table columns are aligned with padding
  - No line wrapping - steps stay on one line
  - Limits consecutive blank lines to maximum 2
  - Writes formatted content back to file

  Core principle: STABILITY
  - Formatting same AST twice produces identical output
  - No "smart" wrapping that causes problems
  - Preserves all content from AST exactly

  Critical implementation requirements:
  - MUST use @cucumber/gherkin parser (official Cucumber parser)
  - MUST support ALL Gherkin elements: Feature, Rule, Background, Scenario, Scenario Outline, Examples
  - MUST support all step keywords: Given, When, Then, And, But, *
  - MUST preserve tags, doc strings, data tables
  - MUST apply consistent indentation
  - MUST align data table columns
  - MUST limit consecutive blank lines to max 2
  - MUST NOT break valid Gherkin syntax
  - SHOULD show which files were formatted
  - Exit code 0 if successful, 1 if errors

  Integration points:
  - CLI command: `fspec format [file]`
  - Can be called before validation/commit
  - Replaces Prettier dependency (bundle size reduction: 199KB → 90KB)
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

  Scenario: Format Scenario Outline with Examples
    Given I have a feature file with Scenario Outline and Examples table
    When I run `fspec format spec/features/outline.feature`
    Then the Scenario Outline should be indented by 2 spaces
    And the Examples section should be indented by 4 spaces
    And the Examples table should be indented by 6 spaces
    And table columns should be aligned with padding

  Scenario: Format Rule keyword
    Given I have a feature file with Rule containing nested Background and Scenarios
    When I run `fspec format spec/features/rules.feature`
    Then the Rule should be indented by 2 spaces
    And nested Background should be indented by 4 spaces
    And nested Scenarios should be indented by 4 spaces
    And steps should be indented by 6 spaces

  Scenario: Format DocStrings with content types
    Given I have a feature file with DocStrings marked as """ json
    When I run `fspec format spec/features/docstrings.feature`
    Then the DocString delimiter should include the content type
    And the content should be preserved exactly
    And both """ and ``` delimiters should be supported

  Scenario: Format multiple feature-level tags
    Given I have a feature with tags @phase1 @cli @formatter
    When I run `fspec format spec/features/tags.feature`
    Then each tag should be on its own line
    And tags should have zero indentation
    And tag order should be preserved

  Scenario: Format scenario-level tags
    Given I have a feature file with scenario-level tags:
      """
      Feature: User Authentication
        @smoke @regression
        Scenario: Login with valid credentials
          Given I am on the login page
          When I enter valid credentials
          Then I should be logged in
      """
    When I run `fspec format spec/features/authentication.feature`
    Then scenario tags should be on separate lines
    And scenario tags should be indented by 2 spaces
    And tag order should be preserved
    And the formatted output should be:
      """
      Feature: User Authentication

        @smoke
        @regression
        Scenario: Login with valid credentials
          Given I am on the login page
          When I enter valid credentials
          Then I should be logged in
      """

  Scenario: Format both feature-level and scenario-level tags
    Given I have a feature file with tags at both levels:
      """
      @phase1 @authentication
      Feature: User Authentication
        @smoke
        Scenario: Login with valid credentials
          Given I am on the login page
          When I enter valid credentials
          Then I should be logged in
        @regression @edge-case
        Scenario: Login with expired session
          Given I have an expired session
          When I attempt to login
          Then I should be prompted to re-authenticate
      """
    When I run `fspec format spec/features/authentication.feature`
    Then feature-level tags should have zero indentation
    And scenario-level tags should be indented by 2 spaces
    And each tag should be on its own line
    And the formatted output should preserve tag hierarchy

  Scenario: Format multiple scenario tags on same line
    Given I have a feature file with multiple scenario tags on one line:
      """
      Feature: Notifications

        @email @sms @push
        Scenario: Send multi-channel notification
          Given I have a message
          When I send notification
          Then user receives it via all channels
      """
    When I run `fspec format spec/features/notifications.feature`
    Then each scenario tag should be separated onto its own line
    And all scenario tags should be indented by 2 spaces
    And the formatted output should be:
      """
      Feature: Notifications

        @email
        @sms
        @push
        Scenario: Send multi-channel notification
          Given I have a message
          When I send notification
          Then user receives it via all channels
      """

  Scenario: Format And/But step keywords
    Given I have a feature file with And and But steps
    When I run `fspec format spec/features/steps.feature`
    Then And steps should be formatted like other steps
    And But steps should be formatted like other steps
    And all steps should be indented by 4 spaces

  Scenario: Format wildcard step keyword
    Given I have a feature file with * step keyword
    When I run `fspec format spec/features/wildcard.feature`
    Then the * steps should be formatted correctly
    And indentation should match other steps

  Scenario: Limit excessive blank lines
    Given I have a feature file with 6 consecutive blank lines in description
    When I run `fspec format spec/features/excessive.feature`
    Then consecutive blank lines should be limited to maximum 2
    And the file should pass validation

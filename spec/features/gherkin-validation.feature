@cli
@parser
@validation
@gherkin
@cucumber-parser
@cross-platform
@critical
@integration-test
Feature: Gherkin Syntax Validation
  """
  Architecture notes:
  - This feature uses @cucumber/gherkin-parser for official Gherkin validation
  - Parser returns AST (Abstract Syntax Tree) or syntax errors
  - Validation is synchronous and fast (no async operations needed)
  - Error messages are formatted for AI agent comprehension
  - Supports all Gherkin keywords: Feature, Background, Scenario, Given, When,
  Then, And, But
  - Validates doc strings (\"\"\"), data tables (|), and tags (@)

  Critical implementation requirements:
  - MUST use @cucumber/gherkin-parser (official Cucumber parser)
  - MUST report line numbers for syntax errors
  - MUST validate ALL .feature files when no specific file provided
  - MUST exit with non-zero code on validation failure
  - Error output MUST be clear enough for AI to self-correct

  Integration points:
  - Called from lifecycle hooks to validate specs before code changes
  - Called from post-commit hooks to validate specs after AI modifications
  - CLI command: `fspec validate [file]`
  - Exit codes: 0 = valid, 1 = syntax errors, 2 = file not found

  References:
  - Gherkin Spec: https://cucumber.io/docs/gherkin/reference
  - Parser Docs: https://github.com/cucumber/gherkin
  - Cucumber Messages: https://github.com/cucumber/messages
  """

  Background: User Story
    As an AI agent writing Gherkin specifications
    I want immediate syntax validation feedback
    So that I can correct errors before committing malformed feature files

  Scenario: Validate a syntactically correct feature file
    Given I have a feature file "spec/features/login.feature" with valid Gherkin syntax
    When I run `fspec validate spec/features/login.feature`
    Then the command should exit with code 0
    And the output should contain "✓ spec/features/login.feature is valid"
    And no error messages should be displayed

  Scenario: Validate all feature files in the project
    Given I have multiple feature files in "spec/features/"
    And all files contain valid Gherkin syntax
    When I run `fspec validate`
    Then the command should exit with code 0
    And the output should list each validated file
    And the output should contain a summary "✓ All N feature files are valid"

  Scenario: Detect missing Feature keyword
    Given I have a file "spec/features/broken.feature" with content:
      """

      User Login

      Scenario: Login successfully
        Given I am on the login page
        When I enter valid credentials
        Then I should be logged in
      """
    When I run `fspec validate spec/features/broken.feature`
    Then the command should exit with code 1
    And the output should contain "Syntax error in spec/features/broken.feature"
    And the output should contain "line 2"
    And the output should contain "Expected: Feature keyword"

  Scenario: Detect invalid step keyword
    Given I have a file "spec/features/broken.feature" with content:
      """
      Feature: User Login

        Scenario: Login successfully
          Given I am on the login page
          While I enter valid credentials
          Then I should be logged in
      """
    When I run `fspec validate spec/features/broken.feature`
    Then the command should exit with code 1
    And the output should contain "Syntax error"
    And the output should contain "line 5"
    And the output should contain "Invalid step keyword 'While'"
    And the output should suggest "Use: Given, When, Then, And, or But"

  Scenario: Detect missing indentation
    Given I have a file "spec/features/broken.feature" with content:
      """
      Feature: User Login

      Scenario: Login successfully
      Given I am on the login page
        When I enter valid credentials
        Then I should be logged in
      """
    When I run `fspec validate spec/features/broken.feature`
    Then the command should exit with code 1
    And the output should contain "Syntax error"
    And the output should contain "Incorrect indentation"

  Scenario: Detect unclosed doc string
    Given I have a file "spec/features/broken.feature" with unclosed doc string
    When I run `fspec validate spec/features/broken.feature`
    Then the command should exit with code 1
    And the output should contain "Unclosed doc string"
    And the output should suggest 'Add closing """'

  Scenario: Validate feature file with doc strings
    Given I have a feature file with properly formatted doc strings
    When I run `fspec validate spec/features/login.feature`
    Then the command should exit with code 0
    And the doc strings should be recognized as valid

  Scenario: Validate feature file with data tables
    Given I have a feature file with properly formatted data tables
    When I run `fspec validate spec/features/login.feature`
    Then the command should exit with code 0
    And the data tables should be recognized as valid

  Scenario: Validate feature file with feature-level tags
    Given I have a feature file with tags at feature level:
      """

      @authentication
      @critical
      Feature: User Login
        Scenario: Successful login
          Given I am on the login page
          When I enter valid credentials
          Then I should be logged in
      """
    When I run `fspec validate spec/features/login.feature`
    Then the command should exit with code 0
    And the feature-level tags should be recognized as valid

  Scenario: Validate feature file with scenario-level tags
    Given I have a feature file with tags at scenario level:
      """
      Feature: User Login

        @smoke
        @regression
        Scenario: Successful login
          Given I am on the login page
          When I enter valid credentials
          Then I should be logged in

        @edge-case
        Scenario: Login with expired session
          Given I have an expired session
          When I attempt to login
          Then I should be prompted to re-authenticate
      """
    When I run `fspec validate spec/features/login.feature`
    Then the command should exit with code 0
    And the scenario-level tags should be recognized as valid
    And each scenario should have its own tags

  Scenario: Validate feature file with both feature-level and scenario-level tags
    Given I have a feature file with tags at both feature and scenario levels:
      """

      @authentication
      Feature: User Login

        @smoke
        Scenario: Successful login
          Given I am on the login page
          When I enter valid credentials
          Then I should be logged in

        @regression
        @edge-case
        Scenario: Login with expired session
          Given I have an expired session
          When I attempt to login
          Then I should be prompted to re-authenticate
      """
    When I run `fspec validate spec/features/login.feature`
    Then the command should exit with code 0
    And both feature-level and scenario-level tags should be recognized as valid
    And tags should be properly associated with their respective elements

  Scenario: Validate scenario-level tags with proper indentation
    Given I have a feature file with scenario tags properly indented:
      """
      Feature: Notifications

        @email
        @sms
        Scenario: Send notification
          Given I have a message
          When I send notification
          Then user receives it
      """
    When I run `fspec validate spec/features/notifications.feature`
    Then the command should exit with code 0
    And the indented scenario tags should be recognized as valid

  Scenario: Detect file not found
    Given no file exists at "spec/features/nonexistent.feature"
    When I run `fspec validate spec/features/nonexistent.feature`
    Then the command should exit with code 2
    And the output should contain "File not found: spec/features/nonexistent.feature"

  Scenario: Validate multiple files and report first error
    Given I have feature files:
      | file                         | status  |
      | spec/features/valid1.feature | valid   |
      | spec/features/broken.feature | invalid |
      | spec/features/valid2.feature | valid   |
    When I run `fspec validate`
    Then the command should exit with code 1
    And the output should contain "✓ spec/features/valid1.feature is valid"
    And the output should contain "✗ spec/features/broken.feature has syntax errors"
    And the output should contain "Validated 3 files: 2 valid, 1 invalid"

  Scenario: Validate with verbose output
    Given I have a valid feature file "spec/features/login.feature"
    When I run `fspec validate --verbose spec/features/login.feature`
    Then the command should exit with code 0
    And the output should contain "Parsing spec/features/login.feature"
    And the output should contain "AST generated successfully"
    And the output should contain the feature name from the file
    And the output should contain the scenario count

  Scenario: AI agent self-correction workflow
    Given I am an AI agent that created a feature file with invalid syntax
    When I run `fspec validate spec/features/my-feature.feature`
    Then I receive a clear error message with line number
    And I receive a suggestion for how to fix the error
    And I can update the file with correct syntax
    And when I run `fspec validate` again, it passes

@phase2
@cli
@feature-management
@modification
@medium
@unit-test
Feature: Add Architecture Documentation to Feature Files
  """
  Architecture notes:
  - Adds or updates architecture doc string in feature files
  - Doc strings use Gherkin triple-quote (\"\"\") syntax
  - Inserted after Feature line, before Background section
  - If architecture doc string exists, it is replaced
  - Preserves existing scenarios and structure
  - Validates Gherkin syntax after modification

  Critical implementation requirements:
  - MUST accept feature file name or path
  - MUST accept architecture text (can be multi-line)
  - MUST use Gherkin doc string syntax (\"\"\")
  - MUST insert after Feature line
  - MUST preserve indentation (2 spaces for doc string content)
  - MUST replace existing architecture doc string if present
  - MUST validate Gherkin syntax after changes
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer documenting feature specifications
    I want to add architecture notes to feature files
    So that implementation details are captured in the specification

  Scenario: Add architecture notes to feature without existing doc string
    Given I have a feature file "login.feature" with no doc string
    When I run `fspec add-architecture login "Uses JWT for authentication"`
    Then the command should exit with code 0
    And the feature file should contain a doc string with "Uses JWT for authentication"
    And the doc string should be after the Feature line
    And the doc string should be before the Background section
    And the file should have valid Gherkin syntax

  Scenario: Add multi-line architecture notes
    Given I have a feature file "api.feature" with no doc string
    When I run `fspec add-architecture api "Architecture notes:\n- Uses REST API\n- Requires authentication"`
    Then the command should exit with code 0
    And the doc string should contain "Architecture notes:"
    And the doc string should contain "- Uses REST API"
    And the doc string should contain "- Requires authentication"
    And the file should have valid Gherkin syntax

  Scenario: Replace existing architecture doc string
    Given I have a feature file "payment.feature" with existing doc string "Old notes"
    When I run `fspec add-architecture payment "New architecture notes"`
    Then the command should exit with code 0
    And the doc string should contain "New architecture notes"
    And the doc string should not contain "Old notes"
    And the file should have valid Gherkin syntax

  Scenario: Add architecture notes preserves scenarios
    Given I have a feature file "checkout.feature" with 3 scenarios
    When I run `fspec add-architecture checkout "Payment processing architecture"`
    Then the command should exit with code 0
    And the feature file should still have 3 scenarios
    And all scenario content should be preserved
    And the file should have valid Gherkin syntax

  Scenario: Add architecture notes preserves Background section
    Given I have a feature file "auth.feature" with Background section
    When I run `fspec add-architecture auth "OAuth 2.0 implementation"`
    Then the command should exit with code 0
    And the Background section should be preserved
    And the doc string should be before the Background section
    And the file should have valid Gherkin syntax

  Scenario: Add architecture notes preserves feature-level tags
    Given I have a feature file "search.feature" with tags "@api @critical"
    When I run `fspec add-architecture search "ElasticSearch integration"`
    Then the command should exit with code 0
    And the feature tags "@api @critical" should be preserved
    And the doc string should be after the Feature line
    And the file should have valid Gherkin syntax

  Scenario: Reject non-existent feature file
    Given I have no feature file named "missing.feature"
    When I run `fspec add-architecture missing "Some notes"`
    Then the command should exit with code 1
    And the output should show "Feature file not found"

  Scenario: Accept feature file by name without .feature extension
    Given I have a feature file "spec/features/login.feature"
    When I run `fspec add-architecture login "Authentication notes"`
    Then the command should exit with code 0
    And the file "spec/features/login.feature" should contain the doc string

  Scenario: Accept feature file by full path
    Given I have a feature file "spec/features/user-management.feature"
    When I run `fspec add-architecture spec/features/user-management.feature "User CRUD operations"`
    Then the command should exit with code 0
    And the file should contain the doc string with "User CRUD operations"

  Scenario: Proper indentation of doc string content
    Given I have a feature file "reporting.feature"
    When I run `fspec add-architecture reporting "Line 1\nLine 2\nLine 3"`
    Then the command should exit with code 0
    And the doc string content should be indented with 2 spaces
    And the opening and closing triple quotes should not be indented
    And the file should have valid Gherkin syntax

  Scenario: Preserve scenario-level tags
    Given I have a feature file "notifications.feature" with scenarios tagged "@email @sms"
    When I run `fspec add-architecture notifications "Notification service architecture"`
    Then the command should exit with code 0
    And the scenario tags "@email @sms" should be preserved
    And the file should have valid Gherkin syntax

  Scenario: Empty architecture text should be rejected
    Given I have a feature file "dashboard.feature"
    When I run `fspec add-architecture dashboard ""`
    Then the command should exit with code 1
    And the output should show "Architecture text cannot be empty"

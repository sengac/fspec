@cli
@feature-management
@modification
@medium
@unit-test
Feature: Add Background Section to Feature Files
  """
  Architecture notes:
  - Adds or updates Background (user story) section in feature files
  - Background uses Gherkin "Background:" keyword
  - Inserted after Feature line and architecture doc string (if present)
  - If Background exists, it is replaced
  - Preserves existing scenarios and structure
  - Validates Gherkin syntax after modification

  Critical implementation requirements:
  - MUST accept feature file name or path
  - MUST accept user story text (As a... I want to... So that...)
  - MUST use Gherkin Background: keyword
  - MUST insert after Feature line and doc string
  - MUST preserve indentation (2 spaces for Background content)
  - MUST replace existing Background if present
  - MUST validate Gherkin syntax after changes
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer documenting feature specifications
    I want to add user story background to feature files
    So that the business context is captured in the specification

  Scenario: Add background to feature without existing background
    Given I have a feature file "login.feature" with no Background section
    When I run `fspec add-background login "As a user\nI want to log in\nSo that I can access my account"`
    Then the command should exit with code 0
    And the feature file should contain "Background: User Story"
    And the background should contain "As a user"
    And the background should contain "I want to log in"
    And the background should contain "So that I can access my account"
    And the Background should be after the Feature line
    And the Background should be before the first Scenario
    And the file should have valid Gherkin syntax

  Scenario: Add background with standard user story format
    Given I have a feature file "search.feature" with no Background section
    When I run `fspec add-background search "As a customer\nI want to search products\nSo that I can find what I need"`
    Then the command should exit with code 0
    And the background should follow the "As a... I want to... So that..." format
    And the file should have valid Gherkin syntax

  Scenario: Replace existing Background section
    Given I have a feature file "checkout.feature" with existing Background "Old story"
    When I run `fspec add-background checkout "As a buyer\nI want to complete checkout\nSo that I can purchase items"`
    Then the command should exit with code 0
    And the background should contain "As a buyer"
    And the background should not contain "Old story"
    And the file should have valid Gherkin syntax

  Scenario: Add background preserves scenarios
    Given I have a feature file "payment.feature" with 3 scenarios
    When I run `fspec add-background payment "As a customer\nI want to pay securely\nSo that my data is protected"`
    Then the command should exit with code 0
    And the feature file should still have 3 scenarios
    And all scenario content should be preserved
    And the file should have valid Gherkin syntax

  Scenario: Add background preserves architecture doc string
    Given I have a feature file "api.feature" with architecture doc string
    When I run `fspec add-background api "As a developer\nI want to use the API\nSo that I can integrate"`
    Then the command should exit with code 0
    And the architecture doc string should be preserved
    And the Background should be after the doc string
    And the file should have valid Gherkin syntax

  Scenario: Add background preserves feature-level tags
    Given I have a feature file "auth.feature" with tags "@security @critical"
    When I run `fspec add-background auth "As a user\nI want authentication\nSo that my data is secure"`
    Then the command should exit with code 0
    And the feature tags "@security @critical" should be preserved
    And the Background should be after the Feature line
    And the file should have valid Gherkin syntax

  Scenario: Reject non-existent feature file
    Given I have no feature file named "missing.feature"
    When I run `fspec add-background missing "As a user..."`
    Then the command should exit with code 1
    And the output should show "Feature file not found"

  Scenario: Accept feature file by name without .feature extension
    Given I have a feature file "spec/features/dashboard.feature"
    When I run `fspec add-background dashboard "As a user\nI want to view dashboard\nSo that I see overview"`
    Then the command should exit with code 0
    And the file "spec/features/dashboard.feature" should contain the Background

  Scenario: Accept feature file by full path
    Given I have a feature file "spec/features/reporting.feature"
    When I run `fspec add-background spec/features/reporting.feature "As a manager\nI want reports\nSo that I track progress"`
    Then the command should exit with code 0
    And the file should contain the Background with "As a manager"

  Scenario: Proper indentation of Background content
    Given I have a feature file "notifications.feature"
    When I run `fspec add-background notifications "As a user\nI want notifications\nSo that I stay informed"`
    Then the command should exit with code 0
    And the Background keyword should not be indented
    And the Background content should be indented with 4 spaces
    And the file should have valid Gherkin syntax

  Scenario: Preserve scenario-level tags
    Given I have a feature file "orders.feature" with scenarios tagged "@smoke @regression"
    When I run `fspec add-background orders "As a customer\nI want to manage orders\nSo that I track purchases"`
    Then the command should exit with code 0
    And the scenario tags "@smoke @regression" should be preserved
    And the file should have valid Gherkin syntax

  Scenario: Empty background text should be rejected
    Given I have a feature file "products.feature"
    When I run `fspec add-background products ""`
    Then the command should exit with code 1
    And the output should show "Background text cannot be empty"

  Scenario: Background positioned after doc string and before scenarios
    Given I have a feature file "integration.feature" with doc string and 2 scenarios
    When I run `fspec add-background integration "As a developer\nI want integration\nSo that systems connect"`
    Then the command should exit with code 0
    And the Background should be after the architecture doc string
    And the Background should be before the first Scenario
    And the file should have valid Gherkin syntax

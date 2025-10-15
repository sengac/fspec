@phase2
@cli
@tag-management
@feature-management
@modification
@medium
@unit-test
Feature: Scenario-Level Tag Management
  """
  Architecture notes:
  - Provides CRUD operations for managing tags at the scenario level
  - Modifies specific scenarios within feature files
  - Uses @cucumber/gherkin parser to parse and reconstruct feature files
  - Validates tag format (@lowercase-with-hyphens) before adding
  - Preserves file structure, formatting, and other content
  - Optionally validates tags against registry (spec/tags.json)

  Critical implementation requirements:
  - MUST preserve existing scenario tags when adding new ones
  - MUST validate tag format before adding
  - MUST prevent duplicate tags on the same scenario
  - MUST maintain tag order (newly added tags appended to end)
  - MUST preserve all other content (feature tags, other scenarios, steps, etc.)
  - MUST validate Gherkin syntax after modifications
  - MUST support scenario identification by name
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer managing test specifications
    I want to add, remove, and list tags on individual scenarios
    So that I can organize test execution without affecting feature-level categorization

  Scenario: Add single tag to scenario
    Given I have a feature file with scenario "Login with valid credentials"
    And the scenario has no tags
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Login with valid credentials" @smoke`
    Then the command should exit with code 0
    And the scenario should have tag @smoke
    And the file should remain valid Gherkin
    And the output should show "Added @smoke to scenario 'Login with valid credentials'"

  Scenario: Add multiple tags to scenario
    Given I have a feature file with scenario "Login with valid credentials"
    And the scenario has tag @smoke
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Login with valid credentials" @critical @regression`
    Then the command should exit with code 0
    And the scenario should have tags @smoke @critical @regression
    And the output should show "Added @critical, @regression to scenario 'Login with valid credentials'"

  Scenario: Prevent adding duplicate tag to scenario
    Given I have a scenario with tags @smoke @critical
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @smoke`
    Then the command should exit with code 1
    And the output should show "Tag @smoke already exists on this scenario"
    And the scenario tags should remain unchanged

  Scenario: Validate tag format when adding to scenario
    Given I have a scenario with tag @smoke
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" InvalidTag`
    Then the command should exit with code 1
    And the output should show "Invalid tag format. Tags must start with @ and use lowercase-with-hyphens"
    And the scenario tags should remain unchanged

  Scenario: Add tag to scenario with registry validation
    Given I have a scenario with tag @smoke
    And the tag @custom-tag is registered in spec/tags.json
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @custom-tag --validate-registry`
    Then the command should exit with code 0
    And the scenario should have tags @smoke @custom-tag

  Scenario: Prevent adding unregistered tag to scenario with validation enabled
    Given I have a scenario with tag @smoke
    And the tag @unregistered is NOT in spec/tags.json
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @unregistered --validate-registry`
    Then the command should exit with code 1
    And the output should show "Tag @unregistered is not registered in spec/tags.json"
    And the scenario tags should remain unchanged

  Scenario: Remove single tag from scenario
    Given I have a scenario with tags @smoke @critical @regression
    When I run `fspec remove-tag-from-scenario spec/features/login.feature "Login" @critical`
    Then the command should exit with code 0
    And the scenario should have tags @smoke @regression
    And the file should remain valid Gherkin
    And the output should show "Removed @critical from scenario 'Login'"

  Scenario: Remove multiple tags from scenario
    Given I have a scenario with tags @smoke @critical @regression @wip
    When I run `fspec remove-tag-from-scenario spec/features/login.feature "Login" @critical @wip`
    Then the command should exit with code 0
    And the scenario should have tags @smoke @regression
    And the output should show "Removed @critical, @wip from scenario 'Login'"

  Scenario: Attempt to remove non-existent tag from scenario
    Given I have a scenario with tags @smoke @regression
    When I run `fspec remove-tag-from-scenario spec/features/login.feature "Login" @critical`
    Then the command should exit with code 1
    And the output should show "Tag @critical not found on this scenario"
    And the scenario tags should remain unchanged

  Scenario: List all scenario-level tags
    Given I have a scenario with tags @smoke @critical @regression @api
    When I run `fspec list-scenario-tags spec/features/login.feature "Login"`
    Then the command should exit with code 0
    And the output should show all tags:
      | @smoke      |
      | @critical   |
      | @regression |
      | @api        |

  Scenario: List tags on scenario with no tags
    Given I have a scenario with no tags
    When I run `fspec list-scenario-tags spec/features/login.feature "Login"`
    Then the command should exit with code 0
    And the output should show "No tags found on this scenario"

  @COV-036
  Scenario: List scenario tags with category information
    Given I have a scenario with tags @smoke @critical @regression
    And the tags are registered in spec/tags.json with categories
    When I run `fspec list-scenario-tags spec/features/login.feature "Login" --show-categories`
    Then the command should exit with code 0
    And the output should show tags with their categories:
      | Tag         | Category       |
      | @smoke      | Test Type Tags |
      | @critical   | Priority Tags  |
      | @regression | Test Type Tags |

  Scenario: Preserve feature-level tags when modifying scenario tags
    Given I have a feature with tags @phase1 @authentication
    And a scenario "Login" with tag @smoke
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @critical`
    Then the command should exit with code 0
    And the scenario should have tags @smoke @critical
    And the feature should still have tags @phase1 @authentication
    And the file should remain valid Gherkin

  Scenario: Preserve other scenarios when modifying one scenario's tags
    Given I have a feature with two scenarios "Login" and "Logout"
    And scenario "Login" has tags @smoke
    And scenario "Logout" has tags @regression
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @critical`
    Then the command should exit with code 0
    And scenario "Login" should have tags @smoke @critical
    And scenario "Logout" should still have tag @regression
    And the file should remain valid Gherkin

  Scenario: Handle scenario not found error
    Given I have a feature file "spec/features/login.feature"
    And the scenario "Nonexistent" does not exist
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Nonexistent" @smoke`
    Then the command should exit with code 1
    And the output should show "Scenario 'Nonexistent' not found in spec/features/login.feature"

  Scenario: Handle file not found error
    Given the file "spec/features/nonexistent.feature" does not exist
    When I run `fspec add-tag-to-scenario spec/features/nonexistent.feature "Login" @smoke`
    Then the command should exit with code 1
    And the output should show "File not found: spec/features/nonexistent.feature"

  Scenario: Remove all tags from scenario
    Given I have a scenario with tags @smoke @critical
    When I run `fspec remove-tag-from-scenario spec/features/login.feature "Login" @smoke @critical`
    Then the command should exit with code 0
    And the scenario should have no tags
    And the file should remain valid Gherkin

  Scenario: Preserve file formatting after scenario tag modification
    Given I have a properly formatted feature file
    And a scenario "Login" with tag @smoke
    When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @critical`
    Then the command should exit with code 0
    And the file formatting should be preserved
    And indentation should remain consistent
    And doc strings should remain intact

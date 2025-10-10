@phase2
@cli
@tag-management
@feature-management
@modification
@medium
@unit-test
Feature: Feature-Level Tag Management
  """
  Architecture notes:
  - Provides CRUD operations for managing tags at the feature level
  - Modifies feature files by adding, removing, or listing tags
  - Uses @cucumber/gherkin parser to parse and reconstruct feature files
  - Validates tag format (@lowercase-with-hyphens) before adding
  - Preserves file structure, formatting, and other content
  - Optionally validates tags against registry (spec/tags.json)

  Critical implementation requirements:
  - MUST preserve existing tags when adding new ones
  - MUST validate tag format before adding
  - MUST prevent duplicate tags
  - MUST maintain tag order (newly added tags appended to end)
  - MUST preserve all other feature file content (scenarios, steps, etc.)
  - MUST validate Gherkin syntax after modifications
  - MUST support both feature-level tags only (scenario tags unchanged)
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer managing feature specifications
    I want to add, remove, and list tags on feature files
    So that I can organize and categorize features without manual editing

  Scenario: Add single tag to feature file
    Given I have a feature file "spec/features/login.feature"
    And the feature has tags @phase1 @authentication
    When I run `fspec add-tag-to-feature spec/features/login.feature @critical`
    Then the command should exit with code 0
    And the feature should have tags @phase1 @authentication @critical
    And the file should remain valid Gherkin
    And the output should show "Added @critical to spec/features/login.feature"

  Scenario: Add multiple tags to feature file
    Given I have a feature file "spec/features/login.feature"
    And the feature has tags @phase1 @authentication
    When I run `fspec add-tag-to-feature spec/features/login.feature @critical @security`
    Then the command should exit with code 0
    And the feature should have tags @phase1 @authentication @critical @security
    And the output should show "Added @critical, @security to spec/features/login.feature"

  Scenario: Prevent adding duplicate tag
    Given I have a feature file with tags @phase1 @critical
    When I run `fspec add-tag-to-feature spec/features/login.feature @critical`
    Then the command should exit with code 1
    And the output should show "Tag @critical already exists on this feature"
    And the feature tags should remain unchanged

  Scenario: Validate tag format when adding
    Given I have a feature file with tags @phase1
    When I run `fspec add-tag-to-feature spec/features/login.feature InvalidTag`
    Then the command should exit with code 1
    And the output should show "Invalid tag format. Tags must start with @ and use lowercase-with-hyphens"
    And the feature tags should remain unchanged

  Scenario: Add tag with registry validation
    Given I have a feature file with tags @phase1
    And the tag @custom-tag is registered in spec/tags.json
    When I run `fspec add-tag-to-feature spec/features/login.feature @custom-tag --validate-registry`
    Then the command should exit with code 0
    And the feature should have tags @phase1 @custom-tag

  Scenario: Prevent adding unregistered tag with validation enabled
    Given I have a feature file with tags @phase1
    And the tag @unregistered is NOT in spec/tags.json
    When I run `fspec add-tag-to-feature spec/features/login.feature @unregistered --validate-registry`
    Then the command should exit with code 1
    And the output should show "Tag @unregistered is not registered in spec/tags.json"
    And the feature tags should remain unchanged

  Scenario: Remove single tag from feature file
    Given I have a feature file with tags @phase1 @critical @authentication
    When I run `fspec remove-tag-from-feature spec/features/login.feature @critical`
    Then the command should exit with code 0
    And the feature should have tags @phase1 @authentication
    And the file should remain valid Gherkin
    And the output should show "Removed @critical from spec/features/login.feature"

  Scenario: Remove multiple tags from feature file
    Given I have a feature file with tags @phase1 @critical @authentication @wip
    When I run `fspec remove-tag-from-feature spec/features/login.feature @critical @wip`
    Then the command should exit with code 0
    And the feature should have tags @phase1 @authentication
    And the output should show "Removed @critical, @wip from spec/features/login.feature"

  Scenario: Attempt to remove non-existent tag
    Given I have a feature file with tags @phase1 @authentication
    When I run `fspec remove-tag-from-feature spec/features/login.feature @critical`
    Then the command should exit with code 1
    And the output should show "Tag @critical not found on this feature"
    And the feature tags should remain unchanged

  Scenario: List all feature-level tags
    Given I have a feature file with tags @phase1 @critical @authentication @api
    When I run `fspec list-feature-tags spec/features/login.feature`
    Then the command should exit with code 0
    And the output should show all tags:
      | @phase1         |
      | @critical       |
      | @authentication |
      | @api            |

  Scenario: List tags on feature with no tags
    Given I have a feature file with no tags
    When I run `fspec list-feature-tags spec/features/login.feature`
    Then the command should exit with code 0
    And the output should show "No tags found on this feature"

  Scenario: List tags with category information
    Given I have a feature file with tags @phase1 @critical @authentication
    And the tags are registered in spec/tags.json with categories
    When I run `fspec list-feature-tags spec/features/login.feature --show-categories`
    Then the command should exit with code 0
    And the output should show tags with their categories:
      | Tag             | Category       |
      | @phase1         | Phase Tags     |
      | @critical       | Priority Tags  |
      | @authentication | Component Tags |

  Scenario: Preserve scenario-level tags when modifying feature tags
    Given I have a feature file with tags @phase1 at feature level
    And scenarios with tags @smoke and @regression at scenario level
    When I run `fspec add-tag-to-feature spec/features/login.feature @critical`
    Then the command should exit with code 0
    And the feature should have tags @phase1 @critical
    And the scenario tags should remain @smoke and @regression
    And the file should remain valid Gherkin

  Scenario: Add tag to feature file without existing tags
    Given I have a feature file with no tags
    When I run `fspec add-tag-to-feature spec/features/login.feature @phase1`
    Then the command should exit with code 0
    And the feature should have tag @phase1
    And the file should remain valid Gherkin

  Scenario: Remove all tags from feature file
    Given I have a feature file with tags @phase1 @critical
    When I run `fspec remove-tag-from-feature spec/features/login.feature @phase1 @critical`
    Then the command should exit with code 0
    And the feature should have no tags
    And the file should remain valid Gherkin

  Scenario: Handle file not found error
    Given the file "spec/features/nonexistent.feature" does not exist
    When I run `fspec add-tag-to-feature spec/features/nonexistent.feature @phase1`
    Then the command should exit with code 1
    And the output should show "File not found: spec/features/nonexistent.feature"

  Scenario: Preserve file formatting after tag modification
    Given I have a properly formatted feature file with tags @phase1
    When I run `fspec add-tag-to-feature spec/features/login.feature @critical`
    Then the command should exit with code 0
    And the file formatting should be preserved
    And indentation should remain consistent
    And doc strings should remain intact

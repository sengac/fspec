@COV-035
@COV-034
@phase7
@cli
@tag-management
@modification
@json-backed
@high
@integration-test
Feature: Register Tag in JSON-Backed Registry
  """
  Architecture notes:
  - Updates existing `fspec register-tag` command to use tags.json
  - Validates tags.json schema before and after modification
  - Automatically regenerates TAGS.md after successful registration
  - Maintains backward compatibility with existing command syntax
  - Ensures atomic operation: validate → modify → regenerate

  Critical implementation requirements:
  - MUST read and parse tags.json
  - MUST validate against tags.schema.json
  - MUST add new tag to appropriate category
  - MUST validate updated JSON against schema
  - MUST regenerate TAGS.md automatically
  - MUST handle errors gracefully (rollback if regeneration fails)
  - MUST preserve all other tags and categories

  Workflow:
  1. Read tags.json
  2. Validate current structure
  3. Add new tag to specified category
  4. Validate modified structure
  5. Write tags.json
  6. Regenerate TAGS.md
  7. Report success

  References:
  - Original command: spec/features/register-tag.feature
  """

  Background: User Story
    As a developer adding new tags to the registry
    I want `fspec register-tag` to edit tags.json and regenerate TAGS.md
    So that the tag registry stays consistent and properly formatted

  Scenario: Register new tag in Phase Tags category
    Given I have a valid file "spec/tags.json"
    When I run `fspec register-tag @phase7 "Phase Tags" "Phase 7: JSON-Backed Documentation"`
    Then the command should exit with code 0
    And "spec/tags.json" should contain the new tag:
      """json
      {
        "name": "@phase7",
        "description": "Phase 7: JSON-Backed Documentation",
        "usage": ""
      }
      """
    And "spec/TAGS.md" should be regenerated
    And the output should display "✓ Registered tag @phase7 in category 'Phase Tags'"
    And the output should display "✓ Regenerated spec/TAGS.md"

  Scenario: Register tag with usage information
    Given I have a valid file "spec/tags.json"
    When I run `fspec register-tag @json-backed "Technical Tags" "JSON-backed documentation features" --usage="Features using JSON schemas and generators"`
    Then "spec/tags.json" should contain:
      """json
      {
        "name": "@json-backed",
        "description": "JSON-backed documentation features",
        "useCases": "Features using JSON schemas and generators"
      }
      """
    And "spec/TAGS.md" should be regenerated

  Scenario: Fail if category does not exist
    Given I have a valid file "spec/tags.json"
    When I run `fspec register-tag @new-tag "Nonexistent Category" "Description"`
    Then the command should exit with code 1
    And the output should display "✗ Category 'Nonexistent Category' not found in tags.json"
    And "spec/tags.json" should not be modified
    And "spec/TAGS.md" should not be regenerated

  Scenario: Fail if tag already exists
    Given I have a valid file "spec/tags.json"
    And the tag "@phase1" already exists
    When I run `fspec register-tag @phase1 "Phase Tags" "Duplicate tag"`
    Then the command should exit with code 1
    And the output should display "✗ Tag @phase1 already exists in registry"
    And "spec/tags.json" should not be modified

  Scenario: Fail if tag name is invalid format
    Given I have a valid file "spec/tags.json"
    When I run `fspec register-tag phase1 "Phase Tags" "Missing @ prefix"`
    Then the command should exit with code 1
    And the output should display "✗ Invalid tag name: must start with @ and contain only lowercase letters, numbers, and hyphens"
    And "spec/tags.json" should not be modified

  Scenario: Rollback if markdown generation fails
    Given I have a valid file "spec/tags.json"
    When I run `fspec register-tag @new-tag "Phase Tags" "Description"`
    And markdown generation fails due to template error
    Then the command should exit with code 1
    And "spec/tags.json" should be rolled back to previous state
    And the output should display "✗ Failed to regenerate TAGS.md - changes rolled back"

  Scenario: Update statistics after registering tag
    Given I have a valid file "spec/tags.json" with current statistics
    When I run `fspec register-tag @new-tag "Technical Tags" "New technical tag"`
    Then "spec/tags.json" statistics should be updated
    And the lastUpdated timestamp should be current
    And "spec/TAGS.md" should reflect updated statistics

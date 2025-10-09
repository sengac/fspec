@phase4
@cli
@tag-management
@modification
@medium
@unit-test
Feature: Update Tag in Registry
  """
  Architecture notes:
  - Modifies existing tag entries in spec/TAGS.md registry
  - Allows updating category and/or description for registered tags
  - Preserves tag name (use retag command to change tag names)
  - Updates tag definition while maintaining file structure
  - Validates tag exists before updating

  Critical implementation requirements:
  - MUST verify tag exists in TAGS.md before updating
  - MUST allow updating category only, description only, or both
  - MUST preserve markdown formatting and structure
  - MUST handle tags in any category
  - MUST provide clear error messages for non-existent tags
  - Exit code 0 for success, 1 for errors
  """

  Background: User Story
    As a developer managing a growing tag taxonomy
    I want to update existing tag definitions (category and description)
    So that I can refine tag organization and documentation without deleting
    and re-creating tags

  Scenario: Update tag description only
    Given I have a tag @phase1 registered in TAGS.md with description "Phase 1 features"
    When I run `fspec update-tag @phase1 --description="Phase 1 - Core validation and feature management"`
    Then the command should exit with code 0
    And the tag @phase1 description should be updated in TAGS.md
    And the tag @phase1 category should remain unchanged
    And the output should show "Successfully updated @phase1"

  Scenario: Update tag category only
    Given I have a tag @deprecated registered in category "Status Tags"
    When I run `fspec update-tag @deprecated --category="Tag Categories"`
    Then the command should exit with code 0
    And the tag @deprecated should be moved to category "Tag Categories"
    And the tag @deprecated description should remain unchanged

  Scenario: Update both category and description
    Given I have a tag @phase1 registered with category "Tag Categories" and description "Phase 1"
    When I run `fspec update-tag @phase1 --category="Tag Categories" --description="Phase 1 - Core validation and feature management"`
    Then the command should exit with code 0
    And the tag @phase1 category should be "Tag Categories"
    And the tag @phase1 description should be "Phase 1 - Core validation and feature management"

  Scenario: Attempt to update non-existent tag
    Given I do not have a tag @nonexistent in TAGS.md
    When I run `fspec update-tag @nonexistent --description="New description"`
    Then the command should exit with code 1
    And the output should show "Tag @nonexistent not found in registry"
    And TAGS.md should remain unchanged

  Scenario: Update tag with invalid category
    Given I have a tag @phase1 registered in TAGS.md
    When I run `fspec update-tag @phase1 --category="Invalid Category"`
    Then the command should exit with code 1
    And the output should show "Invalid category: Invalid Category"
    And the output should list available categories

  Scenario: Update tag without any changes
    Given I have a tag @phase1 registered in TAGS.md
    When I run `fspec update-tag @phase1` without --category or --description
    Then the command should exit with code 1
    And the output should show "No updates specified. Use --category and/or --description"

  Scenario: Update tag preserves other tags
    Given I have multiple tags in TAGS.md
    When I run `fspec update-tag @phase1 --description="Updated description"`
    Then only @phase1 should be modified
    And all other tags should remain unchanged
    And the TAGS.md structure should be preserved

  Scenario: Update tag handles special characters in description
    Given I have a tag @auth registered in TAGS.md
    When I run `fspec update-tag @auth --description="Authentication & authorization with OAuth2.0"`
    Then the command should exit with code 0
    And the description should contain "&" and "2.0"
    And the markdown should be properly escaped

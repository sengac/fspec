@cli
@tag-management
@modification
@medium
@unit-test
Feature: Update Tag in Registry
  """
  Architecture notes:
  - Reads spec/tags.json and updates tag definitions
  - Allows updating category and/or description for registered tags
  - Preserves tag name (use retag command to change tag names)
  - Validates updated JSON against tags.schema.json
  - Regenerates TAGS.md from updated tags.json using generateTagsMd()
  - All updates go through JSON, MD is always auto-generated

  Critical implementation requirements:
  - MUST load tags.json
  - MUST verify tag exists in tags.json before updating
  - MUST allow updating category only, description only, or both
  - MUST validate category name against existing categories
  - MUST update tag object in tags array (or move to new category)
  - MUST maintain alphabetical order within categories
  - MUST validate updated JSON against tags.schema.json
  - MUST write updated tags.json
  - MUST regenerate TAGS.md using generateTagsMd()
  - MUST provide clear error messages for non-existent tags
  - Exit code 0 for success, 1 for errors

  Workflow:
  1. Load spec/tags.json
  2. Validate at least one update specified (category or description)
  3. Find tag in categories array
  4. If category changing, validate new category exists
  5. Update tag object (description and/or move to new category)
  6. Sort tags alphabetically within category
  7. Validate updated JSON against schema
  8. Write spec/tags.json
  9. Regenerate spec/TAGS.md from JSON
  10. Display success message

  References:
  - tags.schema.json: src/schemas/tags.schema.json
  - generateTagsMd(): src/generators/tags-md.ts
  """

  Background: User Story
    As a developer managing a growing tag taxonomy
    I want to update existing tag definitions (category and description)
    So that I can refine tag organization and documentation without deleting
    and re-creating tags

  Scenario: Update tag description only
    Given I have a tag registered in TAGS.md with description "Phase 1 features"
    When I run `fspec update-tag --description="Phase 1 - Core validation and feature management"`
    Then the command should exit with code 0
    And the tag description should be updated in TAGS.md
    And the tag category should remain unchanged
    And the output should show "Successfully updated"

  Scenario: Update tag category only
    Given I have a tag @deprecated registered in category "Status Tags"
    When I run `fspec update-tag @deprecated --category="Tag Categories"`
    Then the command should exit with code 0
    And the tag @deprecated should be moved to category "Tag Categories"
    And the tag @deprecated description should remain unchanged

  Scenario: Update both category and description
    Given I have a tag registered with category "Tag Categories" and description "Phase 1"
    When I run `fspec update-tag --category="Tag Categories" --description="Phase 1 - Core validation and feature management"`
    Then the command should exit with code 0
    And the tag category should be "Tag Categories"
    And the tag description should be "Phase 1 - Core validation and feature management"

  Scenario: Attempt to update non-existent tag
    Given I do not have a tag @nonexistent in TAGS.md
    When I run `fspec update-tag @nonexistent --description="New description"`
    Then the command should exit with code 1
    And the output should show "Tag @nonexistent not found in registry"
    And TAGS.md should remain unchanged

  Scenario: Update tag with invalid category
    Given I have a tag registered in TAGS.md
    When I run `fspec update-tag --category="Invalid Category"`
    Then the command should exit with code 1
    And the output should show "Invalid category: Invalid Category"
    And the output should list available categories

  Scenario: Update tag without any changes
    Given I have a tag registered in TAGS.md
    When I run `fspec update-tag` without --category or --description
    Then the command should exit with code 1
    And the output should show "No updates specified. Use --category and/or --description"

  Scenario: Update tag preserves other tags
    Given I have multiple tags in TAGS.md
    When I run `fspec update-tag --description="Updated description"`
    Then only should be modified
    And all other tags should remain unchanged
    And the TAGS.md structure should be preserved

  Scenario: Update tag handles special characters in description
    Given I have a tag @auth registered in TAGS.md
    When I run `fspec update-tag @auth --description="Authentication & authorization with OAuth2.0"`
    Then the command should exit with code 0
    And the description should contain "&" and "2.0"
    And the markdown should be properly escaped

  Scenario: JSON-backed workflow - modify JSON and regenerate MD
    Given I have a valid tags.json file with @test-tag in "Technical Tags"
    When I run `fspec update-tag @test-tag --description="Updated test tag description"`
    Then the tags.json file should be updated
    And the tags.json should validate against tags.schema.json
    And the @test-tag description in tags.json should be "Updated test tag description"
    And the @test-tag should remain in "Technical Tags" category
    And other tags in tags.json should be preserved
    And TAGS.md should be regenerated from tags.json
    And TAGS.md should contain the updated description
    And TAGS.md should have the auto-generation warning header

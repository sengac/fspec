@phase2
@cli
@file-ops
@tag-management
@critical
Feature: Register New Tag in Tag Registry
  """
  Architecture notes:
  - Reads spec/TAGS.md and updates the appropriate category table
  - Validates that tag follows naming conventions (@lowercase-with-hyphens)
  - Prevents duplicate tag registration
  - Maintains table formatting and alphabetical order within categories
  - Creates TAGS.md if it doesn't exist
  - Supports all tag categories: phase, component, feature-group, technical, platform,








  priority, status, testing, CAGE

  Critical implementation requirements:
  - MUST validate tag format: starts with @, lowercase, hyphens only
  - MUST check for duplicates before adding
  - MUST preserve existing TAGS.md formatting
  - MUST add tags in alphabetical order within their category
  - MUST handle missing TAGS.md gracefully (create with template)
  - Category names must match TAGS.md sections (case-insensitive)

  References:
  - TAGS.md structure: spec/TAGS.md
  - Tag naming conventions: spec/TAGS.md#tag-naming-conventions
  """

  Background: User Story
    As an AI agent extending the fspec tag vocabulary
    I want to register new tags programmatically
    So that I can document and validate custom tags without manual TAGS.md
    editing



  Scenario: Register a new tag in an existing category
    Given I have a TAGS.md file with standard categories
    When I run `fspec register-tag @api "Technical Tags" "API integration features"`
    Then the tag `@api` should be added to the Technical Tags table in TAGS.md
    And the tag should be inserted in alphabetical order
    And the command should confirm the registration with success message

  Scenario: Prevent duplicate tag registration
    Given I have a TAGS.md file with tag `@cli` registered
    When I run `fspec register-tag @cli "Component Tags" "CLI component"`
    Then the command should exit with error code 1
    And the error message should indicate `@cli` is already registered
    And TAGS.md should remain unchanged

  Scenario: Validate tag naming convention
    Given I have a TAGS.md file
    When I run `fspec register-tag InvalidTag "Technical Tags" "Invalid format"`
    Then the command should exit with error code 1
    And the error message should indicate invalid tag format
    And the suggestion should explain valid format: @lowercase-with-hyphens

  Scenario: Create TAGS.md if it doesn't exist
    Given no TAGS.md file exists in spec/
    When I run `fspec register-tag @custom "Technical Tags" "Custom feature"`
    Then a new TAGS.md file should be created with standard structure
    And the tag `@custom` should be added to Technical Tags section
    And the file should include all standard category sections

  Scenario: Register tag with uppercase in input (auto-convert)
    Given I have a TAGS.md file
    When I run `fspec register-tag @API-Integration "Technical Tags" "API features"`
    Then the tag should be registered as `@api-integration` (lowercase)
    And the command should confirm the lowercase conversion
    And TAGS.md should contain `@api-integration`

  Scenario: Handle invalid category name
    Given I have a TAGS.md file
    When I run `fspec register-tag @custom "NonExistent Category" "Description"`
    Then the command should exit with error code 1
    And the error message should list valid categories
    And the suggestion should include available category names from TAGS.md

  Scenario: Preserve TAGS.md formatting when adding tag
    Given I have a TAGS.md file with custom formatting
    When I run `fspec register-tag @new-tag "Technical Tags" "New feature"`
    Then the tag should be added without affecting other sections
    And table column alignment should be preserved
    And existing tags should remain in their original order

  Scenario: Register tag with long description
    Given I have a TAGS.md file
    When I run `fspec register-tag @long-desc "Technical Tags" "This is a very long description that explains the purpose of the tag in detail"`
    Then the tag should be registered with the full description
    And the table should remain properly formatted
    And the description should wrap correctly in the table

  Scenario: AI agent workflow - discover, register, validate
    Given I am an AI agent working on a new feature type
    And I identify a need for tag `@websocket`
    When I run `fspec register-tag @websocket "Technical Tags" "WebSocket communication features"`
    Then the tag should be successfully registered in TAGS.md
    And when I run `fspec validate-tags` on features using `@websocket`
    Then validation should pass for the newly registered tag
    And the tag should appear in `fspec list-tags` output

  Scenario: Register tag in all major categories
    Given I have a TAGS.md file
    When I run `fspec register-tag @phase4 "Phase Tags" "Phase 4: Future features"`
    Then the tag should be added to Phase Tags section
    When I run `fspec register-tag @new-component "Component Tags" "New component"`
    Then the tag should be added to Component Tags section
    When I run `fspec register-tag @new-group "Feature Group Tags" "New feature group"`
    Then the tag should be added to Feature Group Tags section
    And all registrations should maintain proper formatting

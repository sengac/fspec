@phase6
@phase8
@cli
@foundation-management
@modification
@medium
@unit-test
Feature: Update Foundation Section Content
  """
  Architecture notes:
  - Reads spec/foundation.json and updates specified fields
  - Maps section names to JSON structure (e.g., "What We Are Building" -> whatWeAreBuilding.projectOverview)
  - Validates updated JSON against foundation.schema.json
  - Regenerates FOUNDATION.md from updated foundation.json using generateFoundationMd()
  - Creates foundation.json from template if it doesn't exist
  - All updates go through JSON, MD is always auto-generated

  Critical implementation requirements:
  - MUST load foundation.json (or create from template)
  - MUST accept section name and map to JSON field path
  - MUST accept section content (string or object depending on field)
  - MUST update the corresponding JSON field
  - MUST validate updated JSON against foundation.schema.json
  - MUST write updated foundation.json
  - MUST regenerate FOUNDATION.md using generateFoundationMd()
  - Exit code 0 for success, 1 for errors

  Workflow:
  1. Load spec/foundation.json (or create from template if missing)
  2. Validate section name is not empty
  3. Validate content is not empty
  4. Map section name to JSON field path
  5. Update the JSON field with new content
  6. Validate updated JSON against schema
  7. Write spec/foundation.json
  8. Regenerate spec/FOUNDATION.md from JSON
  9. Display success message

  Section name to JSON field mapping:
  - "projectOverview" -> whatWeAreBuilding.projectOverview
  - "coreTechnologies" -> whatWeAreBuilding.technicalRequirements.coreTechnologies (array)
  - "architecture" -> whatWeAreBuilding.technicalRequirements.architecture.pattern
  - "problemDefinition" -> whyWeAreBuildingIt.problemDefinition.primary.description
  - Custom sections may be added as needed

  References:
  - foundation.schema.json: src/schemas/foundation.schema.json
  - generateFoundationMd(): src/generators/foundation-md.ts
  """

  Background: User Story
    As a developer documenting project foundation
    I want to update section content in FOUNDATION.md
    So that I can maintain up-to-date project documentation

  Scenario: Update existing section content
    Given I have a FOUNDATION.md with a "What We Are Building" section
    When I run `fspec update-foundation "What We Are Building" "New content for this section"`
    Then the command should exit with code 0
    And the "What We Are Building" section should contain the new content
    And other sections should be preserved

  Scenario: Create new section if it doesn't exist
    Given I have a FOUNDATION.md without a "Technical Approach" section
    When I run `fspec update-foundation "Technical Approach" "Our technical approach details"`
    Then the command should exit with code 0
    And a new "Technical Approach" section should be created
    And it should contain the specified content

  Scenario: Replace entire section content
    Given I have a "Why" section with existing content
    When I run `fspec update-foundation Why "Completely new reasoning"`
    Then the command should exit with code 0
    And the old content should be completely replaced
    And only the new content should be present in the section

  Scenario: Preserve other sections when updating
    Given I have FOUNDATION.md with "What We Are Building", "Why", and "Architecture" sections
    When I run `fspec update-foundation Why "Updated why section"`
    Then the command should exit with code 0
    And the "What We Are Building" section should be unchanged
    And the "Architecture" section should be unchanged
    And only the "Why" section should have new content

  Scenario: Create FOUNDATION.md if it doesn't exist
    Given I have no FOUNDATION.md file
    When I run `fspec update-foundation "What We Are Building" "A new CLI tool for specifications"`
    Then the command should exit with code 0
    And a FOUNDATION.md file should be created
    And it should contain the "What We Are Building" section
    And the section should have the specified content

  Scenario: Handle multi-line section content
    Given I have a FOUNDATION.md
    When I run `fspec update-foundation Why "Line 1\nLine 2\nLine 3"`
    Then the command should exit with code 0
    And the "Why" section should contain all three lines
    And the lines should be properly formatted

  Scenario: Preserve existing subsections in other sections
    Given I have an "Architecture" section with diagrams (### subsections)
    When I run `fspec update-foundation Why "New content"`
    Then the command should exit with code 0
    And the "Architecture" section diagrams should be preserved
    And only the "Why" section should be modified

  Scenario: Update section at the beginning of file
    Given I have FOUNDATION.md with "Overview" as the first section
    When I run `fspec update-foundation Overview "Updated overview"`
    Then the command should exit with code 0
    And the "Overview" section should have the new content
    And sections after it should be preserved

  Scenario: Update section at the end of file
    Given I have FOUNDATION.md with "Future Plans" as the last section
    When I run `fspec update-foundation "Future Plans" "Updated plans"`
    Then the command should exit with code 0
    And the "Future Plans" section should have the new content
    And sections before it should be preserved

  Scenario: Reject empty section name
    Given I have a FOUNDATION.md
    When I run `fspec update-foundation "" "Some content"`
    Then the command should exit with code 1
    And the output should show "Section name cannot be empty"

  Scenario: Reject empty content
    Given I have a FOUNDATION.md
    When I run `fspec update-foundation Why ""`
    Then the command should exit with code 1
    And the output should show "Section content cannot be empty"

  Scenario: Handle special characters in section names
    Given I have a FOUNDATION.md
    When I run `fspec update-foundation "What We're Building" "Content with apostrophe"`
    Then the command should exit with code 0
    And the section "What We're Building" should be created
    And it should contain the specified content

  Scenario: Preserve markdown formatting in content
    Given I have a FOUNDATION.md
    When I run `fspec update-foundation Features "- Feature 1\n- Feature 2\n- Feature 3"`
    Then the command should exit with code 0
    And the "Features" section should contain a markdown list
    And the list formatting should be preserved

  Scenario: Update section multiple times
    Given I have a "Why" section with content "Original"
    When I run `fspec update-foundation Why "First update"`
    And I run `fspec update-foundation Why "Second update"`
    Then the command should exit with code 0
    And the "Why" section should contain only "Second update"
    And previous content should not be present

  Scenario: JSON-backed workflow - modify JSON and regenerate MD
    Given I have a valid foundation.json file
    When I run `fspec update-foundation projectOverview "Updated project overview content"`
    Then the foundation.json file should be updated
    And the foundation.json should validate against foundation.schema.json
    And the whatWeAreBuilding.projectOverview field should contain "Updated project overview content"
    And other foundation.json fields should be preserved
    And FOUNDATION.md should be regenerated from foundation.json
    And FOUNDATION.md should contain the updated content
    And FOUNDATION.md should have the auto-generation warning header

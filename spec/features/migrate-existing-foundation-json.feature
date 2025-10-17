@migration
@discovery
@foundation
@cli
@phase1
@FOUND-005
Feature: Migrate Existing foundation.json
  """
  Architecture notes:
  - Migration command 'fspec migrate-foundation' reads existing foundation.json
  - Maps legacy v1.x structure to generic v2.0.0 schema (defined in FOUND-001)
  - Uses field mapping rules to transform WHY/WHAT content
  - Preserves architectureDiagrams array as-is (already compatible)
  - HOW content (technical implementation details) extracted to separate docs
  - Validates output using validateGenericFoundationObject from FOUND-001
  - Creates backup of original file before migration
  - Atomic operation: rollback on validation failure
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Must preserve all existing architecture diagrams in architectureDiagrams array
  #   2. WHY/WHAT content stays in foundation.json, HOW content moves to CLAUDE.md or other docs
  #   3. Migration must be automated via CLI command 'fspec migrate-foundation'
  #   4. Migrated foundation.json must pass generic foundation schema validation
  #
  # EXAMPLES:
  #   1. Current 'whatWeAreBuilding.projectOverview' maps to 'project.vision'
  #   2. Current 'whyWeAreBuildingIt.problemDefinition.primary' maps to 'problemSpace.primaryProblem'
  #   3. Current 'architectureDiagrams' array preserved as-is (already compatible)
  #   4. HOW content like 'technicalRequirements.coreTechnologies' moves to CLAUDE.md
  #
  # ========================================
  Background: User Story
    As a developer migrating fspec to generic foundation schema
    I want to migrate existing foundation.json to v2.0.0 format
    So that foundation document follows the new generic schema and supports all project types

  Scenario: Migrate project overview to vision field
    Given I have an existing foundation.json with 'whatWeAreBuilding.projectOverview' field
    When I run 'fspec migrate-foundation'
    Then the new foundation.json should have 'project.vision' field
    And the 'project.vision' should contain the content from 'whatWeAreBuilding.projectOverview'

  Scenario: Migrate problem definition to problem space
    Given I have an existing foundation.json with 'whyWeAreBuildingIt.problemDefinition.primary' field
    When I run 'fspec migrate-foundation'
    Then the new foundation.json should have 'problemSpace.primaryProblem' field
    And the 'problemSpace.primaryProblem' should map from the old structure

  Scenario: Preserve architecture diagrams during migration
    Given I have an existing foundation.json with 'architectureDiagrams' array
    When I run 'fspec migrate-foundation'
    Then the new foundation.json should preserve all diagrams in 'architectureDiagrams' array
    And all diagram sections and titles should remain unchanged

  Scenario: Validate migrated foundation against v2.0.0 schema
    Given I have run 'fspec migrate-foundation' successfully
    When the migration completes
    Then the new foundation.json must pass generic foundation validation
    And validation should use the v2.0.0 schema

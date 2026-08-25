@TOOL-020
Feature: Replace positional `_` args with named keys in fspec tool guidance
  """
  This is a documentation-only change to rust/tools/src/fspec_workflow_guidance.rs - no Rust code changes needed
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every _ positional pattern in fspec_workflow_guidance.rs must be replaced with named keys matching the Rust command Args structs
  #   2. The named keys must use camelCase to match #[serde(rename_all = "camelCase")] on all Rust Args structs
  #   3. The TypeScript callback layer (fspec-callback.ts) must continue to support _ positional args for backward compatibility
  #
  # EXAMPLES:
  #   1. Before: args: {"_": ["AUTH-001", "specifying"]} → After: args: {"workUnitId": "AUTH-001", "status": "specifying"}
  #   2. Before: args: {"_": ["AUTH", "User Login"]} for create-story → After: args: {"prefix": "AUTH", "title": "User Login"}
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec
    I want to call fspec commands with named keys instead of positional args
    So that the tool calls work correctly with both Rust dispatch and TypeScript callback paths

  Scenario: Guidance uses named keys for update-work-unit-status
    Given the fspec workflow guidance file exists at rust/tools/src/fspec_workflow_guidance.rs
    When I inspect the update-work-unit-status examples
    Then I should see "workUnitId" as a named key
    And I should NOT see "_": ["AUTH-001", "specifying"] positional pattern

  Scenario: Guidance uses named keys for show-work-unit
    Given the fspec workflow guidance file exists
    When I inspect the show-work-unit examples
    Then I should see "workUnitId" as a named key
    And I should NOT see "_": ["AUTH-001"] positional pattern

  Scenario: Guidance uses named keys for create-story
    Given the fspec workflow guidance file exists
    When I inspect the create-story examples
    Then I should see "prefix" and "title" as named keys
    And I should NOT see "_": ["AUTH", "User Login"] positional pattern

  Scenario: Guidance uses named keys for add-rule
    Given the fspec workflow guidance file exists
    When I inspect the add-rule examples
    Then I should see "workUnitId" and "rule" as named keys
    And I should NOT see "_": ["AUTH-001", "rule text"] positional pattern

  Scenario: Guidance uses named keys for add-example
    Given the fspec workflow guidance file exists
    When I inspect the add-example examples
    Then I should see "workUnitId" and "example" as named keys
    And I should NOT see "_": ["AUTH-001", "example text"] positional pattern

  Scenario: Guidance uses named keys for add-dependency
    Given the fspec workflow guidance file exists
    When I inspect the add-dependency examples
    Then I should see "workUnitId" and "dependsOn" as named keys
    And I should NOT see "_": ["AUTH-002", "AUTH-001"] positional pattern

  Scenario: Guidance uses named keys for set-user-story
    Given the fspec workflow guidance file exists
    When I inspect the set-user-story examples
    Then I should see "workUnitId" as a named key
    And I should NOT see "_": ["AUTH-001"] positional pattern for work unit ID

  Scenario: Guidance uses named keys for add-attachment
    Given the fspec workflow guidance file exists
    When I inspect the add-attachment examples
    Then I should see "workUnitId" and "filePath" as named keys
    And I should NOT see "_": ["AUTH-001", "file.png"] positional pattern

  Scenario: Guidance uses named keys for link-coverage
    Given the fspec workflow guidance file exists
    When I inspect the link-coverage examples
    Then I should see "feature" as a named key
    And I should NOT see "_": ["user-auth"] positional pattern

  Scenario: Guidance uses named keys for all 80+ command examples
    Given the fspec workflow guidance file exists
    When I count all occurrences of "_": [ positional pattern
    Then the count should be zero
    And every command example should use named keys matching Rust Args structs

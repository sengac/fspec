@work-unit-management
@cli
@BUG-056
Feature: Work unit IDs reused after deletion violating immutability principle
  """
  Bug in generateNextId() function in create-bug.ts, create-story.ts, and create-task.ts. Uses Math.max() on CURRENTLY EXISTING IDs only, ignoring deleted work units. Solution: Add prefixCounters: Record<string, number> to WorkUnitsData type to track high water marks. Similar pattern to nextRuleId/nextExampleId fields within work units (IDX-001). Migration needed to calculate initial high water marks from existing IDs for backward compatibility.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Work unit IDs must never be reused after deletion
  #   2. Each prefix maintains a high water mark counter that never decreases
  #   3. High water mark persists in work-units.json as prefixCounters field
  #   4. Migration calculates initial high water marks from existing IDs for backward compatibility
  #
  # EXAMPLES:
  #   1. Create BUG-001, BUG-002, BUG-003. Delete BUG-003. Next bug gets ID BUG-004 (not BUG-003)
  #   2. Create AUTH-001, DASH-005, TASK-010. High water marks: BUG=0, AUTH=1, DASH=5, TASK=10
  #   3. Load work-units.json without prefixCounters field. Migration scans existing IDs and sets counters automatically
  #   4. Existing project with BUG-054 as highest bug. Migration sets prefixCounters.BUG=54. Next bug gets BUG-055
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to create work units with guaranteed unique IDs
    So that IDs are never reused and git history remains unambiguous

  @critical
  @happy-path
  Scenario: Work unit ID preserved after deletion
    Given I have a project with work-units.json
    And work units "BUG-001", "BUG-002", and "BUG-003" exist
    And the prefixCounters.BUG is 3
    When I delete work unit "BUG-003"
    And I create a new bug with title "New bug"
    Then the new bug should have ID "BUG-004"
    And the prefixCounters.BUG should be 4
    And the work unit "BUG-003" should not exist

  @critical
  @happy-path
  Scenario: Multiple prefixes maintain separate high water marks
    Given I have a project with work-units.json
    When I create work unit "AUTH-001" with type "story"
    And I create work unit "DASH-005" with type "story"
    And I create work unit "TASK-010" with type "task"
    Then the prefixCounters should contain:
      | prefix | counter |
      | AUTH   | 1       |
      | DASH   | 5       |
      | TASK   | 10      |
    When I create a new bug
    And I create a new story with prefix "AUTH"
    And I create a new story with prefix "DASH"
    And I create a new task
    Then the new bug should have ID "BUG-001"
    And the new AUTH story should have ID "AUTH-002"
    And the new DASH story should have ID "DASH-006"
    And the new task should have ID "TASK-011"

  @critical
  @migration
  Scenario: Migration calculates high water marks from existing IDs
    Given I have a work-units.json file without prefixCounters field
    And existing work units:
      | id       | type  |
      | BUG-010  | bug   |
      | BUG-025  | bug   |
      | AUTH-003 | story |
      | DASH-007 | story |
    When the migration system loads work-units.json
    Then the prefixCounters field should be created
    And the prefixCounters should contain:
      | prefix | counter |
      | BUG    | 25      |
      | AUTH   | 3       |
      | DASH   | 7       |
    When I create a new bug
    Then the new bug should have ID "BUG-026"

  @critical
  @backward-compatibility
  Scenario: Backward compatibility with existing projects
    Given I have an existing project with work-units.json
    And the highest bug ID is "BUG-054"
    And the work-units.json file has no prefixCounters field
    When I run migration to add prefixCounters
    Then the prefixCounters.BUG should be set to 54
    When I create a new bug with title "Attachment duplication"
    Then the new bug should have ID "BUG-055"
    And the prefixCounters.BUG should be updated to 55

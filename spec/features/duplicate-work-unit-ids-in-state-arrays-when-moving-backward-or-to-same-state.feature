@cli
@critical
@bug
@state-management
@BUG-064
Feature: Duplicate work unit IDs in state arrays when moving backward or to same state
  """
  The bug exists in src/commands/update-work-unit-status.ts where state array updates fail to remove work unit IDs from all state arrays before adding to the target array. Fix requires: 1) Remove ID from ALL state arrays first, 2) Add ID to target state array, 3) Deduplicate to prevent edge cases.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each work unit ID must appear in exactly one state array at a time
  #   2. Work unit ID must be removed from previous state array before adding to new state array
  #   3. Moving to the same state should be idempotent (no duplicate entries)
  #
  # EXAMPLES:
  #   1. Moving work unit from testing to specifying to testing should result in only one entry in testing array
  #   2. Moving work unit to the same state it's already in should not create a duplicate entry
  #   3. After any state transition, querying the states should return the work unit in only one state
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to move work units backward or to the same state
    So that state arrays remain consistent without duplicates

  Scenario: Moving work unit backward through states should not create duplicates
    Given a work unit TEST-001 exists in testing state
    When I move it to specifying state
    And I move it back to testing state
    Then the testing state array should contain TEST-001 exactly once
    And the specifying state array should not contain TEST-001

  Scenario: Moving work unit to same state should be idempotent
    Given a work unit TEST-002 exists in implementing state
    When I move it to implementing state again
    Then the implementing state array should contain TEST-002 exactly once

  Scenario: Work unit should appear in exactly one state array after any transition
    Given a work unit TEST-003 exists in any state
    When I move it to a different state
    Then TEST-003 should appear in exactly one state array
    And no state array should contain TEST-003 more than once

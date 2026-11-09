@work-unit-management
@cli
@critical
@bug-fix
@stable-indices
@data-integrity
@IDX-002
Feature: Stable Indices Critical Bug Fixes
  """
  Architecture notes:
  - Fixes critical field naming bug: nextNoteId vs nextArchitectureNoteId mismatch
  - Fixes critical data loss bug: state sorting lost during auto-compact
  - Affects: compact-work-unit.ts, update-work-unit-status.ts, kanbanWorkflow.ts
  - Impact: Prevents ID collisions and preserves user-defined sort orders
  - Related: IDX-001 (original stable indices implementation)

  Critical implementation requirements:
  - MUST use 'nextNoteId' consistently across all operations (migration, add, compact, restore)
  - MUST preserve state sorting when auto-compact is triggered
  - MUST NOT re-read from disk after compaction if it discards in-memory state changes
  - Test coverage MUST verify full workflows (compact → add item, sort → compact → verify)
  """

  Background: User Story
    As a developer using stable indices system
    I want to fix critical field naming and data loss bugs
    So that architecture note IDs don't collide and state sorting is preserved during auto-compact

  Scenario: Prevent ID collision after compaction
    Given a work unit AUTH-001 has 5 architecture notes
    And notes at indices 1 and 3 are soft-deleted
    When compact-work-unit removes deleted items and renumbers remaining notes
    And compact-work-unit sets nextNoteId = 3 (correct field)
    And user adds new architecture note "Latest architectural decision"
    Then new note should get id = 3 (not id = 0)
    And no ID collision should occur with existing notes at indices 0, 1, 2

  Scenario: Preserve the done order through auto-compact
    Given work units BOARD-003, BOARD-001 and BOARD-002 are in the done state in that order
    And work unit BOARD-004 is validating with a soft-deleted rule
    When BOARD-004 is moved to done status with auto-compact
    Then the soft-deleted rule of BOARD-004 should be removed
    And the done state array in work-units.json should be BOARD-003, BOARD-001, BOARD-002, BOARD-004

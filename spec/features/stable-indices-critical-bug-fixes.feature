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

  Scenario: Fix field name mismatch in compact-work-unit
    Given compact-work-unit.ts sets architecture note counter on wrong field 'nextArchitectureNoteId'
    And add-architecture-note.ts reads from correct field 'nextNoteId'
    When a work unit is compacted with 3 architecture notes remaining
    Then compact-work-unit.ts should set 'nextNoteId = 3' not 'nextArchitectureNoteId = 3'
    And subsequent add-architecture-note operations should use correct counter
    And no ID collisions should occur

  Scenario: Prevent ID collision after compaction
    Given a work unit AUTH-001 has 5 architecture notes
    And notes at indices 1 and 3 are soft-deleted
    When compact-work-unit removes deleted items and renumbers remaining notes
    And compact-work-unit sets nextNoteId = 3 (correct field)
    And user adds new architecture note "Latest architectural decision"
    Then new note should get id = 3 (not id = 0)
    And no ID collision should occur with existing notes at indices 0, 1, 2

  Scenario: Fix state sorting loss during auto-compact
    Given update-work-unit-status applies state sorting before auto-compact
    And auto-compact saves work unit data to disk
    And system re-reads work-units.json after auto-compact
    When state sorting is applied BEFORE compaction
    Then sorted states are lost because compactWorkUnit doesn't know about them
    And fix should apply state sorting AFTER auto-compact completes

  Scenario: Preserve state sorting through auto-compact
    Given work units BOARD-001, BOARD-002, BOARD-003 in done state
    And done state is sorted by completion time (most recent first)
    When BOARD-004 is moved to done status with auto-compact
    And state sorting is applied AFTER auto-compact (not before)
    Then state sorting should be preserved in final work-units.json
    And done state array should maintain user-defined sort order

  Scenario: Update help documentation for field name consistency
    Given kanbanWorkflow.ts documents 'nextArchitectureNoteId' in stable indices section
    And actual field name in types.ts is 'nextNoteId'
    When help documentation is updated to use correct field name
    Then kanbanWorkflow.ts should document 'nextNoteId' not 'nextArchitectureNoteId'
    And documentation should match implementation

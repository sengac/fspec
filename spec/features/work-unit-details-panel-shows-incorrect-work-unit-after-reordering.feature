@high
@tui
@bug-fix
@interactive-cli
@BOARD-016
Feature: Work unit details panel shows incorrect work unit after reordering
  """
  Architecture notes:
  - Done column work units MUST be sorted in work-units.json states.done array (most recent 'updated' timestamp first)
  - Sorting happens at WRITE time (when moving to done), NOT at display/render time
  - Display order in TUI BoardView MUST match file order from loadData() (no runtime sorting)
  - REFACTOR: Extract shared utility function for states array manipulation (used by moveWorkUnitUp/Down and done sorting)
  - updateWorkUnitStatus command: When transitioning TO done, set 'updated' field and insert ID at correct sorted position
  - Shared function signature: updateStatesArray(workUnitsData, workUnitId, oldStatus, newStatus, sortFn?)

  Critical implementation requirements:
  - MUST set 'updated' field to current ISO timestamp when moving to done
  - MUST SORT ENTIRE states.done array by 'updated' timestamp when ANY work unit moves to done
  - MUST NOT just insert new work unit - MUST fix any existing mis-ordering in done array
  - MUST NOT duplicate array manipulation logic (reuse shared utility)
  - MUST preserve file-based ordering for all other columns (backlog, specifying, testing, implementing, validating, blocked)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Done column work units are sorted by 'updated' timestamp in DESCENDING order (most recent first) IN THE FILE ONLY
  #   2. ALL sorting happens at WRITE time when saving to work-units.json - NEVER at display/render time
  #   3. TUI BoardView MUST display work units in EXACT file order from states.done array (NO runtime sorting)
  #   4. When moving work unit TO done status, ENTIRE states.done array MUST be sorted by 'updated' timestamp and written to file
  #   5. update-work-unit-status command MUST set 'updated' field to current ISO timestamp when transitioning TO done status
  #   6. loadData() in fspecStore reads states.done array in file order and passes to TUI for display
  #   7. NO sorting logic in BoardView, UnifiedBoardLayout, or any TUI component - display uses file order only
  #   8. Use SHARED utility function for inserting/moving work units in states arrays (similar to moveWorkUnitUp/Down) - do NOT duplicate array manipulation logic
  #   9. When moving ANY work unit to done status, the ENTIRE states.done array MUST be sorted by 'updated' timestamp (most recent first), not just the newly inserted work unit
  #   10. Work Unit Details panel selectedWorkUnit MUST match the visual position in the done column (file order = display order)
  #
  # EXAMPLES:
  #   1. Done column has 3 work units: BOARD-001 (updated 10:00), BOARD-002 (updated 11:00), BOARD-003 (updated 09:00). Display order is BOARD-002, BOARD-001, BOARD-003. User selects position 0 (BOARD-002), details panel shows BOARD-002.
  #   2. BOARD-005 currently in implementing status (updated: 2025-10-28T10:00:00Z). User runs 'fspec update-work-unit-status BOARD-005 done'. States.done array has [BOARD-003, BOARD-001]. BOARD-003.updated=11:00, BOARD-001.updated=09:00. BOARD-005 inserted at position 1 (after BOARD-003, before BOARD-001). Final states.done: [BOARD-003, BOARD-005, BOARD-001].
  #   3. Done column states.done array: [BOARD-003, BOARD-005, BOARD-001]. TUI displays in exact file order: position 0=BOARD-003, position 1=BOARD-005, position 2=BOARD-001. User selects position 1, details panel shows BOARD-005 (NOT BOARD-001).
  #   4. Done array has [BOARD-003 (11:00), BOARD-001 (09:00), BOARD-005 (12:00)] in wrong order. When BOARD-007 (10:00) moves to done, ENTIRE array is sorted to [BOARD-005, BOARD-003, BOARD-007, BOARD-001].
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the sorting be applied in the groupedWorkUnits computation in BoardView.tsx, or somewhere else?
  #   A: true
  #
  #   Q: Are there any other columns besides 'done' that need special sorting, or is it only the done column?
  #   A: true
  #
  #   Q: The 'updated' field is optional in the WorkUnit interface - should we handle work units without an 'updated' timestamp? If so, how should they be sorted?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using TUI board
    I want to see correct work unit details for selected item in done column
    So that I can verify completed work matches what's displayed on screen

  Scenario: Done column displays work units sorted by most recent updated timestamp
    Given the done column has 3 work units with updated timestamps:
      | Work Unit | Updated Timestamp    |
      | BOARD-001 | 2025-10-28T10:00:00Z |
      | BOARD-002 | 2025-10-28T11:00:00Z |
      | BOARD-003 | 2025-10-28T09:00:00Z |
    When I view the TUI board
    Then the done column should display work units in order: BOARD-002, BOARD-001, BOARD-003
    And when I select position 0 in done column
    Then the Work Unit Details panel should show BOARD-002 title and description

  Scenario: Moving work unit to done inserts at correct sorted position
    Given BOARD-005 is in implementing status with updated timestamp 2025-10-28T10:00:00Z
    And the states.done array contains [BOARD-003, BOARD-001]
    And BOARD-003 has updated timestamp 2025-10-28T11:00:00Z
    And BOARD-001 has updated timestamp 2025-10-28T09:00:00Z
    When I run "fspec update-work-unit-status BOARD-005 done"
    Then BOARD-005 should be inserted at position 1 in states.done array
    And the final states.done array should be [BOARD-003, BOARD-005, BOARD-001]
    And BOARD-005 updated field should be set to current timestamp

  Scenario: TUI displays done column in exact file array order
    Given the states.done array in work-units.json is [BOARD-003, BOARD-005, BOARD-001]
    When I view the TUI board and navigate to done column
    Then position 0 should display BOARD-003
    And position 1 should display BOARD-005
    And position 2 should display BOARD-001
    And when I select position 1
    Then the Work Unit Details panel should show BOARD-005 (not BOARD-001)

  Scenario: Moving work unit to done sorts entire done array
    Given the states.done array is [BOARD-003, BOARD-001, BOARD-005] (mis-ordered)
    And BOARD-003 has updated timestamp 2025-10-28T11:00:00Z
    And BOARD-001 has updated timestamp 2025-10-28T09:00:00Z
    And BOARD-005 has updated timestamp 2025-10-28T12:00:00Z
    And BOARD-007 is in validating status
    When I run "fspec update-work-unit-status BOARD-007 done"
    Then the ENTIRE states.done array should be re-sorted
    And the final states.done array should be [BOARD-007, BOARD-005, BOARD-003, BOARD-001]
    And BOARD-007 should have the most recent updated timestamp

@BOARD-016
@high
@tui
@interactive-cli
@BOARD-010
Feature: Keyboard priority reordering with bracket keys
  """
  Architecture: CRITICAL ZUSTAND DATA FLOW ISSUE DISCOVERED!

  Key handler flow:
  1. UnifiedBoardLayout useInput detects [ or ] key press
  2. Calls onMoveUp/onMoveDown callback (passed from BoardView as prop)
  3. BoardView callback calls store.moveWorkUnitUp/Down(workUnitId)
  4. Store method swaps IDs in states[columnName] array in work-units.json file
  5. BoardView calls store.loadData() to reload from file
  6. **CRITICAL**: loadData() MUST build workUnits array FROM states arrays!

  WRONG Implementation (current):
  - loadData() does: state.workUnits = Object.values(workUnitsData.workUnits)
  - This loads from workUnits object which has NO ORDER
  - States array changes are ignored!

  CORRECT Implementation (required):
  - loadData() MUST iterate states arrays to build workUnits in order:
  1. For each column (backlog, specifying, testing, etc.)
  2. For each ID in states.columnName array
  3. Look up workUnit details from workUnitsData.workUnits by ID
  4. Push to result array IN ORDER
  - This ensures workUnits array reflects states array order
  - Display order will match states array order

  Key handler architecture:
  - BoardView useInput: High-level keys (ESC, Tab)
  - UnifiedBoardLayout useInput: Navigation keys (arrows, PageUp/Down, Enter, brackets)
  - Pattern: UnifiedBoardLayout receives callbacks as props, calls them on key press

  NO order field - position determined by array index in states[columnName].
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Press [ key to move selected work unit up one position in the current column
  #   2. Press ] key to move selected work unit down one position in the current column
  #   3. Work unit order changes are persisted to work-units.json states array immediately
  #   4. Cannot move work unit up if it's already at the top of the column (first position)
  #   5. Cannot move work unit down if it's already at the bottom of the column (last position)
  #   6. Work unit position is determined by its index in the states[columnName] array in work-units.json
  #   7. To move work unit up: swap its position with the previous work unit ID in the states array
  #   8. To move work unit down: swap its position with the next work unit ID in the states array
  #   9. Manual reordering with [ and ] keys is only allowed in backlog, specifying, testing, implementing, validating, and blocked columns (not done column)
  #  10. Key bindings [ and ] must be displayed in help text at bottom of TUI board
  #  11. When moving work unit up or down, the selection cursor MUST follow the work unit to its new position
  #
  # EXAMPLES:
  #   1. User selects BOARD-002 at position 2, presses [, work unit moves to position 1 AND selection cursor moves to position 1
  #   2. User selects BOARD-001 in backlog column (position 1), presses [, nothing happens (already at top)
  #   3. User selects BOARD-002 at position 2, presses ], work unit moves to position 3 AND selection cursor moves to position 3
  #   4. User selects last work unit in done column, presses ], nothing happens (already at bottom)
  #   5. After moving work unit, order persists when restarting TUI (loaded from work-units.json)
  #   6. User in done column presses [ or ], nothing happens (done column order is automatic by completion time)
  #
  # ========================================
  Background: User Story
    As a developer using TUI board
    I want to reorder work units within a column using bracket keys
    So that I can quickly adjust priorities without leaving the keyboard

  Scenario: Move work unit up one position with [ key
    Given the backlog column has work units in order: BOARD-001, BOARD-002, BOARD-003
    And BOARD-002 is selected at position 2
    When I press the [ key
    Then BOARD-002 should move to position 1
    And the column order should be: BOARD-002, BOARD-001, BOARD-003
    And the selection cursor should be at position 1
    And the order should be persisted to work-units.json

  Scenario: Cannot move work unit up when already at top
    Given the backlog column has work units in order: BOARD-001, BOARD-002, BOARD-003
    And BOARD-001 is selected at position 1
    When I press the [ key
    Then BOARD-001 should remain at position 1
    And the column order should be unchanged: BOARD-001, BOARD-002, BOARD-003
    And no changes should be written to work-units.json

  Scenario: Move work unit down one position with ] key
    Given the backlog column has work units in order: BOARD-001, BOARD-002, BOARD-003
    And BOARD-002 is selected at position 2
    When I press the ] key
    Then BOARD-002 should move to position 3
    And the column order should be: BOARD-001, BOARD-003, BOARD-002
    And the selection cursor should be at position 3
    And the order should be persisted to work-units.json

  Scenario: Cannot move work unit down when already at bottom
    Given the done column has work units in order: EXMAP-001, INIT-001, CLI-001
    And CLI-001 is selected at position 3 (last position)
    When I press the ] key
    Then CLI-001 should remain at position 3
    And the column order should be unchanged: EXMAP-001, INIT-001, CLI-001
    And no changes should be written to work-units.json

  Scenario: Order persists across TUI restarts
    Given I have reordered work units in the backlog column
    And the new order is: BOARD-003, BOARD-001, BOARD-002
    When I exit the TUI and restart it
    Then the backlog column should display work units in the saved order: BOARD-003, BOARD-001, BOARD-002
    And the states.backlog array in work-units.json should reflect the new positions

  Scenario: Manual reordering is disabled in done column
    Given the done column has work units ordered by completion time: BOARD-003, BOARD-005, BOARD-007
    And BOARD-005 is selected at position 2
    When I press the [ key
    Then BOARD-005 should remain at position 2
    And the done column order should be unchanged: BOARD-003, BOARD-005, BOARD-007
    And no changes should be written to work-units.json
    When I press the ] key
    Then BOARD-005 should remain at position 2
    And the done column order should be unchanged: BOARD-003, BOARD-005, BOARD-007
    And no changes should be written to work-units.json

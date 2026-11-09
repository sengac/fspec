@high
@tui
@bug-fix
@interactive-cli
@BOARD-016
Feature: Work unit details panel shows incorrect work unit after reordering
  """
  Architecture notes:
  - The board reads spec/work-units.json through rust/core/src/work_units.rs read_snapshot, which emits each column in its states[<column>] array order (done included); nothing re-sorts a column at display time
  - update-work-unit-status appends a work unit to the end of states.done (update_work_unit_status/board_state.rs move_state); the done column is never re-sorted by updatedAt (H-DONE-ORDER A, product owner, 2026-11-06: append order accepted; BOARD-016's sorting scenarios deleted)
  - Because display order equals file order, the selected position in the done column is the work unit at that position in states.done
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The board shows every column, done included, in the exact order of its states array in work-units.json
  #   2. Nothing sorts the done column by updatedAt, at write time or at display time (H-DONE-ORDER A, product owner, 2026-11-06)
  #
  # EXAMPLES:
  #   1. states.done is [BOARD-003, BOARD-005, BOARD-001] and BOARD-001 was updated last. The done column shows BOARD-003, BOARD-005, BOARD-001.
  #
  # ========================================
  Background: User Story
    As a developer using TUI board
    I want to see correct work unit details for selected item in done column
    So that I can verify completed work matches what's displayed on screen

  Scenario: Board snapshot lists the done column in exact file array order
    Given the states.done array in work-units.json is [BOARD-003, BOARD-005, BOARD-001]
    And BOARD-001 has the most recent updatedAt of the three
    When the board snapshot is read from spec/work-units.json
    Then position 0 of the done column is BOARD-003
    And position 1 of the done column is BOARD-005
    And position 2 of the done column is BOARD-001, not moved first by its updatedAt

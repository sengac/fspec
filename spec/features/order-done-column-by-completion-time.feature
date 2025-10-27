@high
@tui
@board-visualization
@BOARD-011
Feature: Order done column by completion time
  """
  Sort done column work units by extracting last stateHistory entry with state='done' and using its timestamp. Implement sorting in UnifiedBoardLayout or BoardView component before rendering. Use Array.sort() with descending timestamp comparison. Cache sorted order to avoid re-sorting on every render.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Done column displays work units with most recently completed at the top
  #   2. Completion time is determined by the timestamp when work unit was moved to done status (last stateHistory entry with state=done)
  #   3. Work units are sorted in descending order by completion timestamp (newest first)
  #   4. When a work unit is moved back to done from another state, it uses the new timestamp (appears at top again)
  #   5. Other columns (backlog, specifying, testing, implementing, validating, blocked) maintain existing order behavior
  #
  # EXAMPLES:
  #   1. BOARD-005 completed at 10:00, BOARD-003 completed at 11:00, BOARD-007 completed at 09:00 → done column shows: BOARD-003, BOARD-005, BOARD-007
  #   2. BOARD-001 moved to done on Monday, BOARD-002 moved to done on Tuesday → BOARD-002 appears above BOARD-001
  #   3. BOARD-003 completed yesterday, user moves it back to implementing, then back to done today → BOARD-003 now appears at top of done column with today's timestamp
  #   4. On TUI startup, done column loads work units sorted by completion time from stateHistory timestamps
  #
  # ========================================
  Background: User Story
    As a developer viewing the board
    I want to see completed work units ordered by completion time
    So that I can quickly see what was finished most recently

  Scenario: Done column displays work units in descending completion time order
    Given BOARD-005 was moved to done at 10:00 with stateHistory timestamp "2025-10-27T10:00:00Z"
    And BOARD-003 was moved to done at 11:00 with stateHistory timestamp "2025-10-27T11:00:00Z"
    And BOARD-007 was moved to done at 09:00 with stateHistory timestamp "2025-10-27T09:00:00Z"
    When I view the done column
    Then the work units should be displayed in order: BOARD-003, BOARD-005, BOARD-007
    And BOARD-003 should be at the top (most recent completion)
    And BOARD-007 should be at the bottom (oldest completion)

  Scenario: More recently completed work units appear above older ones
    Given BOARD-001 was moved to done on Monday with timestamp "2025-10-21T10:00:00Z"
    And BOARD-002 was moved to done on Tuesday with timestamp "2025-10-22T10:00:00Z"
    When I view the done column
    Then BOARD-002 should appear above BOARD-001
    And the order should reflect completion chronology (newest first)

  Scenario: Re-completing a work unit updates its position to top of done column
    Given BOARD-003 was completed yesterday with timestamp "2025-10-26T10:00:00Z"
    And BOARD-003 is currently in the done column
    When user moves BOARD-003 back to implementing
    And user completes work and moves BOARD-003 back to done today with timestamp "2025-10-27T15:00:00Z"
    Then BOARD-003 should appear at the top of the done column
    And BOARD-003's stateHistory should have a new done entry with today's timestamp
    And the new timestamp should be used for sorting (not the old one)

  Scenario: Done column order persists across TUI restarts
    Given the done column has work units with various completion timestamps
    And the work units are sorted by completion time in the current session
    When I exit the TUI and restart it
    Then the done column should load work units in the same sorted order
    And sorting should be based on stateHistory timestamps from work-units.json
    And most recently completed work units should still appear at the top

  Scenario: Other columns maintain existing order behavior
    Given the backlog column has work units in a specific order
    And the implementing column has work units in a specific order
    When I view the board
    Then the backlog column order should remain unchanged
    And the implementing column order should remain unchanged
    And only the done column should be sorted by completion time

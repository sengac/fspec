@done
@high
@tui
@board
@navigation
@TUI-010
Feature: Mouse Scrolling for Kanban Columns
  """
  Critical: Scroll by 1 item per mouse event (not velocity-based like VirtualList). Respect viewport boundaries: scroll offset min=0, max=(column.length - VIEWPORT_HEIGHT). Existing scroll indicators (↑ ↓) already implemented, no changes needed.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Mouse scrolling applies ONLY to the focused column in the kanban board
  #   2. Mouse scroll down increases the column's scroll offset by 1, showing work units below
  #   3. Mouse scroll up decreases the column's scroll offset by 1, showing work units above
  #   4. Mouse scrolling must parse both raw escape sequences (button codes 96/97) and Ink-parsed mouse events (wheelUp/wheelDown) for terminal compatibility
  #   5. Scroll offset cannot go below 0 or exceed (column length - viewport height)
  #
  # EXAMPLES:
  #   1. User has backlog column focused with 20 work units, viewport shows items 0-9 → user scrolls mouse down → viewport shows items 1-10 (scroll offset becomes 1)
  #   2. User has backlog column focused at scroll offset 5 → user scrolls mouse up → viewport shows items from offset 4 (scrolls up by 1)
  #   3. User at scroll offset 0 (top of column) → user scrolls mouse up → scroll offset remains 0 (cannot scroll above top)
  #   4. User has 15 work units, viewport height is 10, at scroll offset 5 (showing last 10 items) → user scrolls mouse down → scroll offset remains 5 (cannot scroll below bottom)
  #   5. User has backlog column focused and scrolls down to offset 3 → user switches focus to testing column → backlog remains at offset 3, testing starts at offset 0 (per-column scroll offsets)
  #   6. User scrolls backlog column to offset 2 → top row shows ↑ indicator, bottom row shows ↓ indicator (visual feedback that more items exist above and below)
  #   7. User uses Page Down key to scroll backlog to offset 10 → user uses mouse wheel to scroll down by 1 → offset becomes 11 (mouse and keyboard scrolling work together)
  #
  # ========================================
  Background: User Story
    As a user navigating the kanban board
    I want to scroll through work units in columns using mouse wheel or trackpad
    So that I can quickly navigate long lists without using keyboard

  Scenario: Scroll down in focused column with mouse wheel
    Given I am viewing the kanban board
    And the backlog column is focused
    And the column has 20 work units
    And the viewport shows items 0-9 (scroll offset 0)
    When I scroll the mouse wheel down
    Then the scroll offset should increase to 1
    And the viewport should show items 1-10

  Scenario: Scroll up in focused column with mouse wheel
    Given I am viewing the kanban board
    And the backlog column is focused at scroll offset 5
    When I scroll the mouse wheel up
    Then the scroll offset should decrease to 4
    And the viewport should show items from offset 4

  Scenario: Cannot scroll above top of column
    Given I am viewing the kanban board
    And the backlog column is focused at scroll offset 0
    When I scroll the mouse wheel up
    Then the scroll offset should remain 0
    And no scrolling should occur

  Scenario: Cannot scroll below bottom of column
    Given I am viewing the kanban board
    And the backlog column is focused
    And the column has 15 work units
    And the viewport height is 10
    And the scroll offset is 5 (showing last 10 items)
    When I scroll the mouse wheel down
    Then the scroll offset should remain 5
    And no scrolling should occur

  Scenario: Per-column scroll offsets are independent
    Given I am viewing the kanban board
    And the backlog column is focused
    When I scroll down to offset 3 in the backlog column
    And I switch focus to the testing column
    Then the backlog column should remain at offset 3
    And the testing column should start at offset 0

  Scenario: Scroll indicators show when scrolled
    Given I am viewing the kanban board
    And the backlog column is focused
    And the column has more items than the viewport
    When I scroll the backlog column to offset 2
    Then the top row should show the ↑ indicator
    And the bottom row should show the ↓ indicator

  Scenario: Mouse and keyboard scrolling work together
    Given I am viewing the kanban board
    And the backlog column is focused
    When I press Page Down to scroll to offset 10
    And I scroll the mouse wheel down by 1
    Then the scroll offset should be 11

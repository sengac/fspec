@high
@tui
@board-visualization
@bug
@BOARD-012
Feature: Column scrolling tests fail - tests don't trigger scrolling logic
  """
  Component calculates scroll offset using selectedWorkUnitIndex prop. Uses VirtualList pattern from cage project. Scroll indicators (↑↓) consume viewport rows. Each column maintains independent scroll state.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Component must calculate scroll offset based on selectedWorkUnitIndex prop to keep selected item visible
  #   2. When selectedWorkUnitIndex is beyond visible viewport, viewport must scroll to show that item
  #   3. Scroll indicators (↑ ↓) show when there are items above/below visible viewport
  #   4. Scroll indicators consume viewport rows: visible items = VIEWPORT_HEIGHT - number_of_arrows
  #   5. Each column maintains its own scroll offset when switching between columns
  #
  # EXAMPLES:
  #   1. Column has 20 items, viewport height 10, selectedWorkUnitIndex=10 (item 11): viewport scrolls to show items 2-11 with item 11 visible
  #   2. Column has 20 items, viewport height 10, selectedWorkUnitIndex=14 (item 15): viewport scrolls down, item 15 visible, items 1-5 scrolled out
  #   3. Column has 5 items, viewport height 10: no scroll indicators shown, all items visible
  #   4. Column has 20 items, selectedWorkUnitIndex=15: up arrow (↑) shown at top, down arrow (↓) shown at bottom
  #   5. Column has 15 items, viewport height 10, both arrows visible: only 8 items visible (10 - 2 arrows = 8)
  #   6. User at item 15 in column A, switches to column B, then back to column A: scroll position preserved, still at item 15
  #
  # ========================================
  Background: User Story
    As a developer running tests
    I want to verify BoardView scrolling behavior
    So that I can confirm viewport auto-scrolls to keep selected items visible

  Scenario: Auto-scroll viewport when selected item is beyond visible range
    Given I have a column with 20 work items
    And the viewport height is 10 items
    When selectedWorkUnitIndex is set to 10 (item 11)
    Then the viewport should scroll to show items 2-11
    And item 11 should be visible and selected

  Scenario: No scroll indicators when all items fit in viewport
    Given I have a column with 5 work items
    And the viewport height is 10 items
    When the component renders
    Then no scroll indicators should be shown
    And all 5 items should be visible

  Scenario: Show scroll indicators when items exceed viewport
    Given I have a column with 20 work items
    And the viewport height is 10 items
    When selectedWorkUnitIndex is set to 15
    Then an up arrow indicator should appear at the top
    And a down arrow indicator should appear at the bottom

  Scenario: Scroll indicators consume viewport rows
    Given I have a column with 15 work items
    And the viewport height is 10 items
    When both scroll indicators are visible
    Then only 8 work items should be visible
    And the selected item should remain visible

  Scenario: Preserve scroll position when switching columns
    Given I am viewing item 15 in column A
    When I switch to column B
    And then switch back to column A
    Then the scroll position should be preserved
    And item 15 should still be visible and selected

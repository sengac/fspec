@done
@phase-1
@board-visualization
@tui
@board
@BUG-045
Feature: Work items in columns don't scroll
  """
  Critical requirements: Scroll indicators (↑ ↓) must only show when items exceed viewport height. Wrap-around navigation required at boundaries. Scroll position must persist per column when switching columns (preserve user context).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Arrow key navigation (up/down) should automatically scroll the viewport when selection moves beyond visible area
  #   2. Scrolling should follow the selected item, keeping it visible at all times
  #   3. Implementation should match VirtualList pattern from cage project (navigateTo helper function)
  #   4. Scroll indicators (↑ ↓) should show when there are items above/below visible viewport
  #   5. No, only arrow keys should trigger automatic scrolling. Do not use j/k vim-style navigation.
  #   6. Preserve scroll position per column. Each column maintains its own scroll offset state.
  #   7. Yes, wrap-around navigation should work (down on last item goes to first, up on first item goes to last).
  #
  # EXAMPLES:
  #   1. User presses down arrow 11 times in a column with 20 items (viewport shows 10). Selection moves to item 11, viewport scrolls to show items 2-11 with selection visible.
  #   2. User at item 15 presses up arrow to item 5. Viewport scrolls up to show items 1-10 with item 5 selected.
  #   3. Column with 5 items and viewport height 10: no scroll indicators show, all items visible.
  #   4. Column with 20 items: ↓ indicator shows at bottom row when scrollOffset < (20 - 10), ↑ indicator shows at top row when scrollOffset > 0.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should vim-style navigation (j/k keys) also trigger automatic scrolling, matching the cage VirtualList pattern?
  #   A: No, only arrow keys should be used. No j/k vim-style navigation.
  #
  #   Q: When switching columns with left/right arrows, should we reset scroll offset to 0 for the new column, or preserve scroll position per column?
  #   A: Preserve scroll position per column. Each column maintains its own scroll offset state.
  #
  #   Q: Should wrap-around navigation work (pressing down on last item goes to first item)?
  #   A: Yes, wrap-around navigation should work (down on last item goes to first, up on first item goes to last).
  #
  # ========================================
  Background: User Story
    As a user navigating the fspec board TUI
    I want to scroll through work items using arrow keys
    So that I can see all work items in a column without manually pressing Page Up/Down

  Scenario: Automatic scroll when navigating beyond visible viewport
    Given I am viewing a column with 20 work items
    And the viewport shows 10 items at a time
    And I am at item 1
    When I press down arrow 11 times
    Then the selection should move to item 11
    And the viewport should scroll to show items 2-11
    And item 11 should be visible and selected

  Scenario: Scroll up when navigating backward through items
    Given I am viewing a column with 20 work items
    And the viewport shows 10 items at a time
    And I am at item 15
    When I press up arrow 10 times to reach item 5
    Then the viewport should scroll up to show items 1-10
    And item 5 should be visible and selected

  Scenario: No scroll indicators when all items fit in viewport
    Given I am viewing a column with 5 work items
    And the viewport height is 10 items
    When I render the board
    Then no scroll indicators should be displayed
    And all 5 items should be visible

  Scenario: Show scroll indicators when items exceed viewport
    Given I am viewing a column with 20 work items
    And the viewport height is 10 items
    When the scroll offset is greater than 0
    Then an up arrow indicator (↑) should appear at the top
    When the scroll offset is less than maximum (20 - 10)
    Then a down arrow indicator (↓) should appear at the bottom

  Scenario: Wrap-around navigation at column boundaries
    Given I am viewing a column with 10 work items
    And I am at the last item (item 10)
    When I press down arrow
    Then the selection should wrap to the first item (item 1)
    And the viewport should scroll to show the first item
    Given I am at the first item (item 1)
    When I press up arrow
    Then the selection should wrap to the last item (item 10)
    And the viewport should scroll to show the last item

  Scenario: Preserve scroll position when switching columns
    Given I am in column A at item 15 with scroll offset 10
    When I press right arrow to switch to column B
    Then column A should remember scroll offset 10
    And when I return to column A
    Then the scroll offset should still be 10
    And item 15 should still be selected

  Scenario: Account for scroll indicators when calculating visible items
    Given I am viewing a column with 15 work items
    And the viewport height is 10 items
    And I am at item 8 (middle of the list)
    When the board renders with both up and down arrows visible
    Then the up arrow should appear at row 0
    And the down arrow should appear at row 9
    And only 8 work items should be visible (rows 1-8)
    And the selected item should remain visible and not be hidden by arrows

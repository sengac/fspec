@navigation
@high
@tui
@keyboard-navigation
@BUG-062
Feature: Kanban TUI keyboard navigation - Page Up/Down and Home/End key bindings

  """
  Root cause in UnifiedBoardLayout.tsx:430-458 - Page Up/Down directly manipulate scrollOffsets instead of calling onWorkUnitChange
  Missing Home/End key bindings in UnifiedBoardLayout.tsx useInput handler (lines 404-478)
  Fix: Change Page Down to call onWorkUnitChange(VIEWPORT_HEIGHT), Page Up to call onWorkUnitChange(-VIEWPORT_HEIGHT)
  Fix: Add Home key handler to call onWorkUnitChange with movement to index 0, End key to call onWorkUnitChange with movement to last index
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Page Up/Down should move the selector (selectedWorkUnitIndex) by VIEWPORT_HEIGHT, not manipulate scrollOffsets directly
  #   2. Home/End keys must be bound in UnifiedBoardLayout to move selector to first/last item in current column
  #   3. Auto-scroll logic (lines 320-391 in UnifiedBoardLayout) automatically adjusts scrollOffsets to keep selector visible
  #
  # EXAMPLES:
  #   1. User on item 5 of 20, presses Page Down, selector moves to item 15 (assuming VIEWPORT_HEIGHT=10), list scrolls to show items around 15
  #   2. User on item 15, presses Page Up, selector moves to item 5, list scrolls to show items around 5
  #   3. User on item 15, presses Home, selector moves to item 0 (first item), list scrolls to top
  #   4. User on item 5, presses End, selector moves to item 19 (last item of 20), list scrolls to bottom
  #
  # QUESTIONS (ANSWERED):
  #   Q: When you press Page Down, what currently happens? Does the selector stay in the same visual position on screen while the list scrolls, or does the selector jump down but the list doesn't scroll to follow it?
  #   A: true
  #
  #   Q: What is the desired Page Down behavior? Should the selector move down one full page of items (e.g., if 10 items visible, move selector down 10 items), and the list scrolls to keep selector visible?
  #   A: true
  #
  #   Q: For Home/End keys - the code shows they ARE bound (key.home/key.end) but also 'g' and 'G' are vim-style alternatives. Are Home/End keys not working for you, or is it a different issue?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a user navigating the Kanban TUI
    I want to use Page Up/Down and Home/End keys
    So that I can quickly navigate through long lists without my muscle memory being broken

  Scenario: Page Down moves selector down one page
    Given I am viewing a Kanban column with 20 work units
    And the selector is on item 5
    And the viewport height is 10 items
    When I press Page Down
    Then the selector should move to item 15
    And the list should scroll to show items around item 15

  Scenario: Page Up moves selector up one page
    Given I am viewing a Kanban column with 20 work units
    And the selector is on item 15
    And the viewport height is 10 items
    When I press Page Up
    Then the selector should move to item 5
    And the list should scroll to show items around item 5

  Scenario: Home key moves selector to first item
    Given I am viewing a Kanban column with 20 work units
    And the selector is on item 15
    When I press Home
    Then the selector should move to item 0
    And the list should scroll to the top

  Scenario: End key moves selector to last item
    Given I am viewing a Kanban column with 20 work units
    And the selector is on item 5
    When I press End
    Then the selector should move to item 19
    And the list should scroll to the bottom

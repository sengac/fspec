@tui-component
@ui-enhancement
@done
@tui
@TUI-032
Feature: VirtualList scroll-only mode for AgentModal

  """
  Testing: Existing VirtualList tests must continue passing (no changes to item mode behavior)
  Testing: Add new test file VirtualList-scroll-mode.test.tsx for scroll mode specific behavior
  Implementation:
  - Add selectionMode?: 'item' | 'scroll' prop to VirtualListProps interface with default value 'item'
  - Create scrollTo(offset: number) function for direct scrollOffset manipulation in scroll mode
  - In scroll mode, keyboard handler uses scrollTo() instead of navigateTo() for arrow/page keys
  - In scroll mode, mouse scroll handler calls scrollTo() directly instead of navigateTo()
  - Compute maxScrollOffset = Math.max(0, items.length - visibleHeight) for scroll clamping
  - In render loop, pass isSelected = (selectionMode === 'item' && actualIndex === selectedIndex) to renderItem
  - Disable onFocus effect in scroll mode by adding condition: if (selectionMode === 'item') { onFocus?.(...) }
  - Update scrollToEnd effect to only set scrollOffset (not selectedIndex) when selectionMode === 'scroll'
  - Remove the 'adjust scroll offset to keep selected item visible' effect in scroll mode (selection drives scroll in item mode only)
  Refactoring:
  - Extract navigation logic into two separate handlers: handleItemNavigation() for item mode, handleScrollNavigation() for scroll mode
  UI/UX:
  - UI/UX: In AgentModal, remove cyan highlighting since isSelected is always false in scroll mode - use role-based colors only
  Dependency:
  - Files to modify: src/tui/components/VirtualList.tsx, src/tui/components/AgentModal.tsx
  - Files that must NOT change behavior: CheckpointViewer.tsx, FileDiffViewer.tsx, ChangedFilesViewer.tsx (use default item mode)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. VirtualList must support two selection modes: 'item' (default, current behavior) and 'scroll' (new, viewport-only scrolling)
  #   2. In 'scroll' mode, arrow keys (up/down) directly adjust scrollOffset by 1 line without changing selectedIndex
  #   3. In 'scroll' mode, PageUp/PageDown directly adjust scrollOffset by visibleHeight lines
  #   4. In 'scroll' mode, Home/End scroll to top/bottom of content
  #   5. In 'scroll' mode, mouse scroll directly adjusts scrollOffset (with acceleration) without changing selectedIndex
  #   6. In 'scroll' mode, isSelected parameter passed to renderItem must always be false
  #   7. In 'scroll' mode, onFocus callback must never be called since no item is focused
  #   8. In 'scroll' mode, scrollToEnd prop must still work by setting scrollOffset to show the last item
  #   9. In 'scroll' mode, scrollOffset must be clamped to valid range [0, max(0, items.length - visibleHeight)]
  #   10. In 'item' mode (default), all existing VirtualList behavior must remain unchanged for backwards compatibility
  #   11. AgentModal must use selectionMode='scroll' for the conversation VirtualList
  #   12. CheckpointViewer and FileDiffViewer must continue using default 'item' mode for file/checkpoint selection
  #
  # EXAMPLES:
  #   1. VirtualList with selectionMode='scroll' and 100 lines, user presses down arrow, scrollOffset increases by 1 and no line is highlighted
  #   2. VirtualList with selectionMode='scroll' showing 20 lines, user presses PageDown, scrollOffset increases by 20
  #   3. VirtualList with selectionMode='scroll' and scrollOffset=50, user presses Home, scrollOffset becomes 0
  #   4. VirtualList with selectionMode='scroll' and 100 items, user presses End, scrollOffset becomes max position to show last item
  #   5. VirtualList with selectionMode='scroll', user scrolls mouse wheel down rapidly, scrollOffset increases with acceleration (up to 5 lines per scroll)
  #   6. VirtualList with selectionMode='scroll', renderItem receives isSelected=false for every item regardless of scroll position
  #   7. VirtualList with selectionMode='scroll' and onFocus callback provided, callback is never invoked during scrolling
  #   8. VirtualList with selectionMode='scroll' and scrollToEnd=true, when new items are added the view auto-scrolls to show the last item
  #   9. VirtualList with selectionMode='scroll' at scrollOffset=0, user presses up arrow, scrollOffset remains 0 (clamped)
  #   10. VirtualList with selectionMode='scroll' at max scrollOffset, user presses down arrow, scrollOffset remains at max (clamped)
  #   11. AgentModal conversation area uses VirtualList with selectionMode='scroll', chat messages scroll smoothly without individual line selection
  #   12. VirtualList with default selectionMode (item mode), arrow keys move selection through items one by one with cyan highlighting (existing behavior preserved)
  #   13. CheckpointViewer uses VirtualList without selectionMode prop (defaults to item mode), file selection works as before
  #
  # ========================================

  Background: User Story
    As a developer using fspec TUI
    I want to scroll through chat content in AgentModal without individual line selection
    So that I get a natural chat-reading experience where content scrolls as a unit

  # ========================================
  # SCROLL MODE SCENARIOS
  # ========================================

  @scroll-mode
  Scenario: Scroll down with arrow key in scroll mode
    Given a VirtualList with selectionMode set to "scroll"
    And the list contains 100 lines of content
    And the current scroll offset is 0
    When the user presses the down arrow key
    Then the scroll offset should increase by 1
    And no line should be highlighted

  @scroll-mode
  Scenario: Scroll up with arrow key in scroll mode
    Given a VirtualList with selectionMode set to "scroll"
    And the list contains 100 lines of content
    And the current scroll offset is 50
    When the user presses the up arrow key
    Then the scroll offset should decrease by 1
    And no line should be highlighted

  @scroll-mode
  Scenario: Page down scrolls by visible height
    Given a VirtualList with selectionMode set to "scroll"
    And the list contains 100 lines of content
    And the visible height is 20 lines
    And the current scroll offset is 0
    When the user presses PageDown
    Then the scroll offset should increase by 20

  @scroll-mode
  Scenario: Page up scrolls by visible height
    Given a VirtualList with selectionMode set to "scroll"
    And the list contains 100 lines of content
    And the visible height is 20 lines
    And the current scroll offset is 40
    When the user presses PageUp
    Then the scroll offset should decrease by 20

  @scroll-mode
  Scenario: Home key scrolls to top
    Given a VirtualList with selectionMode set to "scroll"
    And the list contains 100 lines of content
    And the current scroll offset is 50
    When the user presses the Home key
    Then the scroll offset should become 0

  @scroll-mode
  Scenario: End key scrolls to bottom
    Given a VirtualList with selectionMode set to "scroll"
    And the list contains 100 lines of content
    And the visible height is 20 lines
    When the user presses the End key
    Then the scroll offset should become the maximum position to show the last item

  @scroll-mode
  Scenario: Mouse scroll with acceleration
    Given a VirtualList with selectionMode set to "scroll"
    And the list contains 100 lines of content
    When the user scrolls the mouse wheel down rapidly within 150ms intervals
    Then the scroll offset should increase with acceleration up to 5 lines per scroll event

  @scroll-mode
  Scenario: isSelected is always false in scroll mode
    Given a VirtualList with selectionMode set to "scroll"
    And the list contains 50 lines of content
    When renderItem is called for any item
    Then the isSelected parameter should be false for all items

  @scroll-mode
  Scenario: onFocus callback is never invoked in scroll mode
    Given a VirtualList with selectionMode set to "scroll"
    And an onFocus callback is provided
    When the user scrolls through the content
    Then the onFocus callback should never be invoked

  @scroll-mode
  Scenario: scrollToEnd auto-scrolls to last item
    Given a VirtualList with selectionMode set to "scroll"
    And scrollToEnd is set to true
    And the list contains 50 lines of content
    When new items are added to the list
    Then the view should auto-scroll to show the last item

  @scroll-mode
  Scenario: Scroll offset clamped at top boundary
    Given a VirtualList with selectionMode set to "scroll"
    And the list contains 100 lines of content
    And the current scroll offset is 0
    When the user presses the up arrow key
    Then the scroll offset should remain 0

  @scroll-mode
  Scenario: Scroll offset clamped at bottom boundary
    Given a VirtualList with selectionMode set to "scroll"
    And the list contains 100 lines of content
    And the visible height is 20 lines
    And the current scroll offset is at the maximum position
    When the user presses the down arrow key
    Then the scroll offset should remain at the maximum position

  # ========================================
  # ITEM MODE SCENARIOS (BACKWARDS COMPATIBILITY)
  # ========================================

  @item-mode
  Scenario: Arrow keys move selection in item mode
    Given a VirtualList with default selectionMode
    And the list contains 50 items
    And the first item is selected
    When the user presses the down arrow key
    Then the second item should be selected
    And the second item should be highlighted with cyan color

  @item-mode
  Scenario: CheckpointViewer uses item mode by default
    Given the CheckpointViewer component is rendered
    And it uses VirtualList without specifying selectionMode
    When the user navigates through checkpoints with arrow keys
    Then individual checkpoints should be selectable
    And the selected checkpoint should be highlighted

  # ========================================
  # AGENTMODAL INTEGRATION SCENARIOS
  # ========================================

  @integration
  Scenario: AgentModal uses scroll mode for conversation
    Given the AgentModal is open
    And it contains a conversation with multiple messages
    When the user scrolls through the conversation
    Then the content should scroll smoothly without individual line selection
    And no lines should show selection highlighting
    And messages should display with role-based colors only
@done
@tui
@tui-component
@ui-enhancement
@TUI-041
Feature: Toggle line selection mode with /select command
  """
  Implementation Notes:
  - State: Add isLineSelectMode boolean to AgentView (default false = scroll mode)
  - Command: Parse /select in handleSubmit alongside /debug, /clear, /resume, /search
  - VirtualList: Pass selectionMode={isLineSelectMode ? 'item' : 'scroll'}
  - UI: Add [SELECT] indicator in header bar when isLineSelectMode is true (cyan color)
  - Visual: Update renderItem to use isSelected param for cyan highlight with > prefix
  - Files: src/tui/components/AgentView.tsx only (VirtualList unchanged)
  - Dependency: VirtualList already supports selectionMode prop (TUI-032)

  Smart Scrolling Behavior:
  - Scroll mode: Has sticky scroll (userScrolledAway tracking) - scroll away and new messages don't auto-scroll
  - Line selection mode: Always auto-selects last line when new messages arrive (no sticky scroll)
  - This is intentional: line selection is for quick copy operations, scroll mode is for reading
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The /select command toggles between two modes: scroll mode (default, current behavior) and line selection mode (new)
  #   2. In scroll mode (default), arrow keys scroll the viewport without selecting individual lines (isSelected always false)
  #   3. In line selection mode, arrow keys navigate between individual lines with visual selection indicator (cyan highlight)
  #   4. Smart scrolling (scrollToEnd with sticky behavior) must be preserved in scroll mode - auto-scroll to new messages unless user has scrolled away
  #   5. In line selection mode, new messages always auto-select the last line (no sticky scroll - intentional for quick copy operations)
  #   6. A [SELECT] indicator must appear in the header bar when line selection mode is active (similar to [DEBUG] indicator)
  #   7. The toggle state persists until explicitly toggled again or the AgentView is closed
  #
  # EXAMPLES:
  #   1. User types '/select' in input, mode toggles to line selection, [SELECT] indicator appears, confirmation message shows 'Line selection mode enabled'
  #   2. User types '/select' again to toggle back, mode returns to scroll-only, [SELECT] indicator disappears, message shows 'Line selection mode disabled'
  #   3. In scroll mode, user presses down arrow 3 times, conversation scrolls down 3 lines, no line is highlighted
  #   4. In line selection mode, user presses down arrow 3 times, selection moves down 3 lines with cyan highlighting
  #   5. In line selection mode with auto-scroll enabled, new message arrives, selection automatically moves to the new last line
  #   6. In scroll mode, user scrolls up to read older messages, new message arrives but view stays at current position (sticky scroll)
  #   7. In scroll mode, user scrolls back to bottom of conversation, auto-scroll re-enables for new messages
  #   8. AgentView starts in scroll mode by default (no [SELECT] indicator), matching current behavior
  #
  # ========================================
  Background: User Story
    As a developer using fspec AgentView
    I want to toggle between scroll-only mode and line selection mode using /select command
    So that I can either smoothly scroll through conversation content like a chat, or select individual lines to copy or reference them

  @command
  Scenario: Enable line selection mode with /select command
    Given AgentView is open in scroll mode
    And the header bar does not show a SELECT indicator
    When I type /select and press Enter
    Then line selection mode should be enabled
    And a SELECT indicator should appear in the header bar
    And the conversation should show a confirmation message

  @command
  Scenario: Disable line selection mode with /select command
    Given AgentView is open in line selection mode
    And the header bar shows a SELECT indicator
    When I type /select and press Enter
    Then line selection mode should be disabled
    And the SELECT indicator should disappear from the header bar
    And scroll mode should be restored

  @scroll-mode
  Scenario: Arrow keys scroll viewport in scroll mode
    Given AgentView is open in scroll mode
    And the conversation contains multiple messages
    When I press the down arrow key 3 times
    Then the conversation should scroll down 3 lines
    And no line should be highlighted

  @selection-mode
  Scenario: Arrow keys select lines in line selection mode
    Given AgentView is open in line selection mode
    And the conversation contains multiple messages
    When I press the down arrow key 3 times
    Then the selection should move down 3 lines
    And the selected line should be highlighted with cyan color

  @selection-mode
  Scenario: Auto-scroll to new messages in line selection mode
    Given AgentView is open in line selection mode
    And the conversation is scrolled to the bottom
    When a new message arrives from the assistant
    Then the selection should automatically move to the new last line
    And the new message should be visible

  @scroll-mode
  @smart-scroll
  Scenario: Sticky scroll when user scrolls away in scroll mode
    Given AgentView is open in scroll mode
    And the conversation has been receiving messages
    When I scroll up to read older messages
    And a new message arrives from the assistant
    Then the view should stay at the current position
    And the new message should not be auto-scrolled to

  @scroll-mode
  @smart-scroll
  Scenario: Re-enable auto-scroll when scrolling back to bottom in scroll mode
    Given AgentView is open in scroll mode
    And I have scrolled away from the bottom of the conversation
    And auto-scroll is temporarily disabled
    When I scroll back to the bottom of the conversation
    Then auto-scroll should be re-enabled
    And new messages should auto-scroll into view

  @default
  Scenario: AgentView starts in scroll mode by default
    When I open the AgentView for the first time
    Then the conversation should be in scroll mode
    And no SELECT indicator should be visible in the header bar
    And arrow keys should scroll the viewport without selecting lines

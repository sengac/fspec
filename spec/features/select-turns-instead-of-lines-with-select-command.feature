@done
@tui
@tui-component
@ui-enhancement
@TUI-042
Feature: Select turns instead of lines with /select command
  """
  Implementation Notes:
  - This REPLACES TUI-041's line-based selection with turn-based selection. Same /select command, upgraded semantics.
  - Leverage existing messageIndex field in ConversationLine to identify turns.
  - State: Add selectedTurnIndex (number | null). Rename/reuse isLineSelectMode as isTurnSelectMode.
  - turnBoundaries memo: Compute array of { turnIndex, firstLineIndex, lastLineIndex } from conversationLines.
  - VirtualList: Use selectionMode='scroll' when turn selection active. Set isFocused=false to disable VL input.
  - Navigation: Add useInput in AgentView to intercept arrow keys when isTurnSelectMode is true.
  - Scrolling: Add scrollToIndex prop to VirtualList for programmatic scrolling to selected turn.
  - Rendering: In renderItem, check line.messageIndex === selectedTurnIndex for cyan highlight + > prefix.
  - Files: src/tui/components/AgentView.tsx, src/tui/components/VirtualList.tsx
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The /select command toggles turn selection mode - when enabled, arrow keys navigate between entire conversation turns (messages) rather than individual lines
  #   2. When a turn is selected, ALL lines belonging to that turn (same messageIndex) are highlighted with cyan color and > prefix
  #   3. A turn consists of a single conversation message (user prompt, assistant response, or tool output) which may span multiple display lines
  #   4. When entering turn selection mode, the last turn in the conversation is automatically selected
  #   5. The viewport automatically scrolls to show the first line of the selected turn when navigating
  #   6. Up arrow on the first turn stays on the first turn (no wrap-around), down arrow on the last turn stays on the last turn
  #   7. The [SELECT] indicator in the header bar shows when turn selection mode is active
  #
  # EXAMPLES:
  #   1. User types /select, mode enables, [SELECT] indicator appears, last turn (all its lines) is highlighted in cyan with > prefix
  #   2. Conversation has 5 turns, user enables turn selection (last turn selected), presses up arrow twice, third turn is now selected with all its lines highlighted
  #   3. Assistant response spans 15 lines, when that turn is selected, all 15 lines show > prefix and cyan highlighting
  #   4. User types /select again while in turn selection mode, mode disables, [SELECT] indicator disappears, no turns are highlighted
  #   5. First turn is selected, user presses up arrow, selection stays on first turn (does not wrap to last turn)
  #   6. Turn is scrolled out of view, user navigates to it with arrow keys, viewport scrolls to show the start of that turn
  #   7. AgentView opens, no [SELECT] indicator visible, arrow keys scroll the viewport (default scroll mode)
  #   8. Tool output message appears (e.g., 'Turn selection mode enabled'), it becomes its own selectable turn separate from user/assistant messages
  #
  # ========================================
  Background: User Story
    As a developer using fspec AgentView
    I want to select entire conversation turns (messages) with /select command
    So that I can navigate between and copy complete user/assistant exchanges rather than individual lines

  @command
  @turn-selection
  Scenario: Enable turn selection mode with /select command
    # Example [0]: User types /select, mode enables, [SELECT] indicator appears, last turn (all its lines) is highlighted in cyan with > prefix
    Given AgentView is open in scroll mode
    And the conversation contains multiple turns
    And the header bar does not show a SELECT indicator
    When I type /select and press Enter
    Then turn selection mode should be enabled
    And a SELECT indicator should appear in the header bar
    And the last turn in the conversation should be selected
    And all lines of the selected turn should be highlighted in cyan with > prefix

  @navigation
  @turn-selection
  Scenario: Navigate between turns with arrow keys
    # Example [1]: Conversation has 5 turns, user enables turn selection (last turn selected), presses up arrow twice, third turn is now selected with all its lines highlighted
    Given AgentView is open with 5 conversation turns
    And turn selection mode is enabled
    And the last turn is currently selected
    When I press the up arrow key twice
    Then the third turn should be selected
    And all lines of the third turn should be highlighted in cyan with > prefix
    And the previously selected turn should no longer be highlighted

  @highlighting
  @turn-selection
  Scenario: Multi-line turn highlighting
    # Example [2]: Assistant response spans 15 lines, when that turn is selected, all 15 lines show > prefix and cyan highlighting
    Given AgentView is open with turn selection mode enabled
    And there is an assistant response that spans 15 lines
    When I navigate to select that turn
    Then all 15 lines of the turn should show the > prefix
    And all 15 lines should be displayed in cyan color

  @command
  @turn-selection
  Scenario: Disable turn selection mode with /select command
    # Example [3]: User types /select again while in turn selection mode, mode disables, [SELECT] indicator disappears, no turns are highlighted
    Given AgentView is open in turn selection mode
    And the header bar shows a SELECT indicator
    And a turn is currently selected and highlighted
    When I type /select and press Enter
    Then turn selection mode should be disabled
    And the SELECT indicator should disappear from the header bar
    And no turns should be highlighted
    And scroll mode should be restored

  @navigation
  @boundary
  Scenario: Navigation stays at first turn when pressing up
    # Example [4]: First turn is selected, user presses up arrow, selection stays on first turn (does not wrap to last turn)
    Given AgentView is open with turn selection mode enabled
    And the first turn in the conversation is selected
    When I press the up arrow key
    Then the first turn should remain selected
    And the selection should not wrap to the last turn

  @navigation
  @boundary
  Scenario: Navigation stays at last turn when pressing down
    # Rule [5]: Down arrow on the last turn stays on the last turn (no wrap-around)
    Given AgentView is open with turn selection mode enabled
    And the last turn in the conversation is selected
    When I press the down arrow key
    Then the last turn should remain selected
    And the selection should not wrap to the first turn

  @scrolling
  @turn-selection
  Scenario: Viewport scrolls to show selected turn
    # Example [5]: Turn is scrolled out of view, user navigates to it with arrow keys, viewport scrolls to show the start of that turn
    Given AgentView is open with turn selection mode enabled
    And there are more turns than fit in the viewport
    And a turn near the bottom is currently selected
    When I press the up arrow key to select a turn that is scrolled out of view
    Then the viewport should scroll to show the selected turn
    And the first line of the selected turn should be visible

  @default
  @scroll-mode
  Scenario: AgentView starts in scroll mode by default
    # Example [6]: AgentView opens, no [SELECT] indicator visible, arrow keys scroll the viewport (default scroll mode)
    When I open the AgentView for the first time
    Then the conversation should be in scroll mode
    And no SELECT indicator should be visible in the header bar
    And arrow keys should scroll the viewport without selecting turns

  @turn-selection
  @tool-messages
  Scenario: Tool messages are selectable as separate turns
    # Example [7]: Tool output message appears (e.g., 'Turn selection mode enabled'), it becomes its own selectable turn separate from user/assistant messages
    Given AgentView is open with turn selection mode enabled
    And the conversation contains user messages, assistant responses, and tool output messages
    When I navigate through the turns with arrow keys
    Then tool output messages should be selectable as their own separate turns
    And each tool message turn should be highlighted independently when selected

@agent-interaction
@done
@tui
@virtuallist
@TUI-043
Feature: Expand command for turn-based tool output expansion

  """
  Architecture notes:
  - File: src/tui/components/AgentView.tsx
  - State: Add expandedMessageIndices (Set<number>) to track which message indices are expanded
  - Data model: Add fullContent optional field to ConversationMessage interface for storing uncollapsed content
  - VirtualList integration: Add selectionRef prop to VirtualList.tsx to expose current selectedIndex to parent
  - Tool result handler: Store both collapsed (content) and full (fullContent) versions when processing ToolResult events
  - conversationLines computation: Check expandedMessageIndices and swap content/fullContent accordingly
  - Cache invalidation: Delete lineCacheRef entry for message when expansion state toggles
  - Hint message update: Change '(ctrl+o to expand)' to '(use /select and /expand)' in formatCollapsedOutput, formatDiffForDisplay, formatDiffForDisplayReadTool functions
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. /expand command only works when turn selection mode is active (after /select)
  #   2. /expand toggles expansion state for the currently selected turn
  #   3. Expanded turn shows full tool output without truncation or '...+N lines' hint
  #   4. Collapsed output hint message changed from 'ctrl+o to expand' to 'use /select and /expand'
  #   5. Expansion state persists when navigating between turns
  #   6. /expand shows error message if not in turn selection mode
  #
  # EXAMPLES:
  #   1. User runs /select, navigates to a collapsed tool output turn, runs /expand → turn expands to show full output
  #   2. User runs /expand on already expanded turn → turn collapses back to truncated view
  #   3. User runs /expand without first enabling /select → error message 'Must be in turn selection mode. Use /select first.'
  #   4. Tool output with 50 lines shows '...+46 lines (use /select and /expand)' instead of old 'ctrl+o to expand'
  #   5. Edit tool diff output with 100 lines shows collapsed view, /expand shows full diff with all changes visible
  #   6. User expands turn A, navigates to turn B (collapsed), navigates back to turn A → turn A is still expanded
  #
  # ========================================

  Background: User Story
    As a developer using the AI agent TUI
    I want to toggle expansion of collapsed tool output for a selected turn
    So that I can see the full output without scrolling through the collapsed content hints

  Scenario: Expand collapsed tool output in turn selection mode
    Given I have a conversation with a tool output turn showing "...+46 lines (use /select and /expand)"
    And I run the /select command to enable turn selection mode
    And I navigate to the collapsed tool output turn
    When I run the /expand command
    Then the turn expands to show the full tool output without truncation
    And the "...+N lines" hint is no longer visible for that turn


  Scenario: Collapse expanded turn by running expand again
    Given I am in turn selection mode with a turn currently expanded
    When I run the /expand command
    Then the turn collapses back to the truncated view
    And the "...+N lines (use /select and /expand)" hint is visible again


  Scenario: Silently ignore expand when not in turn selection mode
    Given I am NOT in turn selection mode
    When I run the /expand command
    Then the command does nothing silently


  Scenario: Updated hint message in collapsed tool output
    Given a tool produces output with 50 lines
    When the output is displayed in the conversation
    Then I see the hint "...+46 lines (use /select and /expand)"
    And I do NOT see the old hint "ctrl+o to expand"


  Scenario: Expand collapsed diff output from Edit tool
    Given I have a conversation with an Edit tool diff showing 100 lines of changes collapsed
    And I am in turn selection mode with that turn selected
    When I run the /expand command
    Then the full diff is visible with all 100 lines of changes


  Scenario: Expansion state persists when navigating between turns
    Given I am in turn selection mode
    And I have expanded turn A using /expand
    And turn B is still collapsed
    When I navigate to turn B
    And then navigate back to turn A
    Then turn A is still expanded
    And turn B remains collapsed


@done
@interactive-cli
@tui
@TUI-056
Feature: Fix Anchor Viewer Integration with Thinking Dialog System
  """
  Update AgentView.tsx disabled prop logic to coordinate multiple dialog states properly
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Anchor viewer dialog must not interfere with existing ThinkingLevelDialog state management
  #   2. Modal dialog state must be properly coordinated to prevent multiple overlays
  #   3. Anchor points must be accessible from TUI via rustStateSource without breaking existing state management
  #
  # EXAMPLES:
  #   1. When user types /anchors command, anchor viewer opens without disrupting other UI dialogs like model selector or settings
  #   2. When anchor viewer displays anchor points, it shows data from current session without causing TUI to freeze or crash
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the anchor viewer be a modal dialog, a side panel, or a separate view mode to avoid conflicts with thinking dialog?
  #   A: Modal dialog to match existing UI patterns like ThinkingLevelDialog. Add proper state coordination in AgentView.tsx to prevent multiple overlays by updating the disabled prop logic.
  #
  #   Q: What anchor data should be exposed through rustStateSource.getAnchorPoints() function that was referenced in the failed attempt?
  #   A: Expose array of AnchorPoint objects with {turn_index, anchor_type, confidence, description, timestamp} properties. Function should filter by session ID and return anchors detected during compaction for that session.
  #
  # ========================================
  Background: User Story
    As a user interacting with TUI
    I want to view anchor points in conversation without breaking existing dialog system
    So that I can inspect context compaction decisions without UI conflicts

  Scenario: Anchor viewer opens without disrupting existing UI dialogs
    Given I am in the TUI with an active session
    And other dialogs like model selector or settings may be available
    When I type the /anchors command
    Then the anchor viewer opens as a modal dialog
    And it does not disrupt or interfere with other UI dialogs

  Scenario: Anchor viewer displays session data without causing crashes
    Given I am in the TUI with an active session containing anchor points
    When the anchor viewer displays anchor points from the current session
    Then it shows the anchor data correctly formatted
    And the TUI does not freeze or crash
    And the anchor data is filtered to the current session only

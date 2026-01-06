@tui-component
@agent-modal
@TUI-045
Feature: Select mode enter key opens full turn modal
  """
  Architecture Notes:
  1. Use existing VirtualList onSelect prop - VirtualList already calls onSelect on Enter in item mode (line 473-474)
  2. Extract renderConversationLine function from inline renderItem to share diff coloring logic between main VirtualList and modal VirtualList
  3. Modal positioning: Use Dialog component pattern with position='absolute' width='100%' height='100%' for full-screen overlay
  4. Modify existing Esc handler (line 4028-4039) to check showTurnModal and isTurnSelectMode BEFORE existing checks for isLoading and inputValue
  5. Remove /expand command handler block (lines 1249-1281) and expandedMessageIndices state (line 605) and its usage in conversationLines useMemo
  6. Create modalLines useMemo that wraps (fullContent || content) for modalMessageIndex using similar logic to wrapMessageToLines but without role prefix or separator
  7. Update formatCollapsedOutput (line 352-364) to change hint from '(select turn to /expand)' to '(Enter to view full)'
  8. Update main VirtualList isFocused prop to include '&& !showTurnModal' to prevent keyboard capture when modal is open (line 5022)
  9. Modal keyboard handling: Use main useInput with priority check (showTurnModal checked first in Esc handler)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing Enter in select mode opens the selected turn's full content (fullContent field) in a modal dialog
  #   2. The modal displays the turn content in a scrollable VirtualList (not collapsed/truncated)
  #   3. Pressing Esc while the modal is open closes the modal and returns to select mode
  #   4. Pressing Esc while in select mode (and modal is NOT open) disables select mode
  #   5. The /expand command is removed - Enter key in modal view replaces it entirely
  #   6. The expandedMessageIndices state is removed - no longer needed for inline expansion
  #   7. VirtualList onSelect callback is used to open the modal when Enter is pressed in select mode (no custom Enter key handler needed)
  #   8. Modal state consists of showTurnModal boolean and modalMessageIndex number to track which conversation message is displayed
  #   9. Main VirtualList isFocused must include '&& !showTurnModal' to prevent input capture when modal is open
  #   10. The collapse hint text changes from '(select turn to /expand)' to '(Enter to view full)' in formatCollapsedOutput function
  #   11. Modal VirtualList uses selectionMode='scroll' with scrollToEnd=false to start at top and allow free scrolling
  #   12. Esc key priority order: 1) close modal, 2) disable select mode, 3) interrupt loading, 4) clear input, 5) exit view
  #   13. Diff coloring logic ([R]/[A] markers, context lines) is shared between main view and modal via extracted renderConversationLine function
  #   14. Modal content is prepared by wrapping (fullContent || content) into ConversationLine[] with word-wrapping for terminal width
  #
  # EXAMPLES:
  #   1. User presses Tab to enter select mode, navigates to an assistant turn with truncated tool output, presses Enter - modal opens showing full untruncated content
  #   2. User has modal open showing a long tool output, presses Esc - modal closes and user is back in select mode with same turn selected
  #   3. User is in select mode with no modal open, presses Esc - select mode is disabled and user returns to normal conversation view
  #   4. User selects a user turn (green 'You:' message), presses Enter - modal shows the full user message content scrollable
  #   5. Modal shows full content with proper diff coloring preserved (red for removed, green for added lines) from Edit tool outputs
  #   6. User is NOT in select mode, presses Esc - normal behavior (interrupt/clear/exit) occurs, no select mode toggle
  #   7. Modal shows title indicating message role: 'User Message', 'Assistant Response', or 'Tool Output'
  #   8. Modal footer shows navigation hints: '↑↓ Scroll | Esc Close'
  #   9. User selects a short assistant text response (no truncation), presses Enter - modal shows the same content since fullContent is undefined, content field is used
  #   10. Collapsed tool output in main view shows '... +42 lines (Enter to view full)' instead of old /expand hint
  #
  # ========================================
  Background: User Story
    As a user in select mode navigating turns in AgentView
    I want to press Enter to open the selected turn in a full-screen scrollable modal, and use Esc to close the modal or exit select mode
    So that I can view large turns (with long tool outputs) in their entirety in a dedicated modal view, replacing the inline /expand command

  @happy-path
  Scenario: Open truncated tool output in modal via Enter key
    Given I am in AgentView with a conversation containing truncated tool output
    And I press Tab to enter select mode
    And I navigate to the assistant turn with truncated tool output
    When I press Enter
    Then a modal dialog opens showing the full untruncated content
    And the modal content is scrollable via VirtualList

  @happy-path
  Scenario: Close modal with Esc returns to select mode
    Given I am in AgentView in select mode
    And I have opened a turn in the modal view
    When I press Esc
    Then the modal closes
    And I am still in select mode with the same turn selected

  @happy-path
  Scenario: Exit select mode with Esc when modal is not open
    Given I am in AgentView in select mode
    And no modal is open
    When I press Esc
    Then select mode is disabled
    And I return to normal conversation view

  @happy-path
  Scenario: Open user message in modal
    Given I am in AgentView with a conversation containing a user message
    And I press Tab to enter select mode
    And I navigate to the user turn
    When I press Enter
    Then the modal opens showing the full user message content
    And the modal title indicates "User Message"

  @edge-case
  Scenario: Diff coloring preserved in modal for Edit tool output
    Given I am in AgentView with a conversation containing Edit tool output with diff
    And I press Tab to enter select mode
    And I navigate to the tool output turn
    When I press Enter
    Then the modal shows the full diff content
    And removed lines have red background
    And added lines have green background

  @edge-case
  Scenario: Esc behavior when not in select mode clears input
    Given I am in AgentView not in select mode
    And there is text in the input field
    When I press Esc
    Then the input field is cleared
    And select mode is not toggled

  @ui
  Scenario Outline: Modal displays role-specific title for <role> messages
    Given I am in AgentView in select mode
    And I navigate to a <role> turn
    When I press Enter to open the modal
    Then the modal title shows "<expected_title>"

    Examples:
      | role      | expected_title     |
      | user      | User Message       |
      | assistant | Assistant Response |
      | tool      | Tool Output        |

  @ui
  Scenario: Modal displays navigation hints in footer
    Given I am in AgentView in select mode
    And I have opened any turn in the modal
    When I look at the modal footer
    Then it shows "↑↓ Scroll | Esc Close"

  @edge-case
  Scenario: Open short message with no fullContent falls back to content
    Given I am in AgentView with a short assistant text response that was not truncated
    And the message has no fullContent field
    And I press Tab to enter select mode
    And I navigate to that assistant turn
    When I press Enter
    Then the modal opens showing the content field value
    And the modal is still scrollable

  @ui
  Scenario: Collapsed output shows updated hint text
    Given I am in AgentView with a conversation containing a tool output with more than 4 lines
    When the tool output is displayed in collapsed form
    Then the collapse indicator shows "... +N lines (Enter to view full)"
    And the old "/expand" hint text is not present

  @integration
  Scenario: Esc key closes modal before disabling select mode
    Given I am in AgentView in select mode with a modal open
    When I press Esc
    Then the modal closes
    And select mode remains active
    When I press Esc again
    Then select mode is disabled

  @removal
  Scenario: The /expand command is no longer recognized
    Given I am in AgentView in select mode with a turn selected
    When I type "/expand" and press Enter
    Then the command is not recognized as a special command
    And it is sent as a regular message to the agent

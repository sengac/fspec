@done
@TUI-057
Feature: Refactor Anchor Viewer from Dialog to Full-Screen View

  """
  Create AnchorView.tsx as full-screen component following WatcherCreateView pattern with position='absolute' and terminal dimensions
  Use useInputCompat with CRITICAL priority and return true for ALL input at end of handler to ensure complete input isolation
  Implement split pane layout similar to SplitSessionView: left pane for anchor VirtualList, right pane for AnchorTurnPreview component
  Delete AnchorViewerDialog.tsx after migration complete - no backward compatibility needed
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Anchor view must be full-screen and take over terminal like watcher view pattern
  #   2. All input must be consumed by the view - no keystrokes should leak to underlying components
  #   3. View must show split pane layout: anchor list on left, turn details preview on right
  #   4. All references to AnchorViewerDialog must be removed from AgentView.tsx and any other files - no deprecated imports, state variables, or JSX
  #   5. No TODO, DEPRECATED, or migration-related comments should remain in the codebase after implementation
  #   6. Clean implementation: no comments referencing 'old dialog', 'migration from', 'refactor', or 'replaced' - code should read as if AnchorView was the original design
  #   7. This story is read-only viewing - editing/adding/deleting anchors is out of scope (future enhancement)
  #   8. Keyboard navigation: arrow keys navigate anchor list, Esc exits view - no Enter action needed since preview is always visible
  #   9. NO EMOJIS - use text labels and ASCII characters only for anchor type indicators and status
  #   10. Left pane must show rich metadata per anchor: anchor type label, turn number, confidence score, timestamp, and any available context summary
  #
  # EXAMPLES:
  #   1. User navigates with ↑↓ in anchor list → right pane updates to show selected anchor's turn content (user message, assistant response, tool calls)
  #   2. User presses Esc → anchor view closes and returns to AgentView
  #   3. User types random keys while in anchor view → no keys leak to AgentView underneath, all input consumed by view
  #   4. User types /anchors → full-screen view opens showing anchor list in left pane, selected anchor's turn details in right pane
  #   5. User types /anchors on session with no anchors → view opens with empty state message 'No anchor points found in this session'
  #   6. User types /anchors with no active session → status message 'Start a session first to view anchor points' (same as current behavior)
  #
  # ========================================

  Background: User Story
    As a developer using the TUI
    I want to view anchor points in a dedicated full-screen view
    So that I have complete input isolation and more space for anchor details

  @happy-path
  Scenario: Open anchor view with /anchors command
    Given I have an active session with anchor points
    When I type "/anchors"
    Then a full-screen anchor view opens
    And the left pane shows the anchor list with metadata
    And the right pane shows the selected anchor's turn content
    And the view has a header showing "Conversation Anchors"
    And the footer shows available keyboard shortcuts

  Scenario: Navigate anchors with arrow keys
    Given the anchor view is open with multiple anchors
    When I press the down arrow key
    Then the next anchor in the list is selected
    And the right pane updates to show that anchor's turn content
    When I press the up arrow key
    Then the previous anchor in the list is selected
    And the right pane updates to show that anchor's turn content

  Scenario: Anchor list shows rich metadata
    Given the anchor view is open with anchors
    Then each anchor item displays the anchor type label
    And each anchor item displays the turn number
    And each anchor item displays the confidence score
    And each anchor item displays the relative timestamp
    And no emoji characters are used in the display

  Scenario: Preview pane shows turn content
    Given the anchor view is open with an anchor selected
    Then the right pane shows the user message for that turn
    And the right pane shows the assistant response for that turn
    And the right pane shows any tool calls made in that turn
    And the content is scrollable with a scrollbar indicator

  Scenario: Exit anchor view with Escape
    Given the anchor view is open
    When I press the Escape key
    Then the anchor view closes
    And I return to the AgentView

  Scenario: All input is consumed by anchor view
    Given the anchor view is open
    When I type random characters
    Then no keystrokes leak to components underneath
    And the AgentView does not receive any input

  Scenario: Empty state when no anchors exist
    Given I have an active session with no anchor points
    When I type "/anchors"
    Then the anchor view opens
    And a message displays "No anchor points found in this session"

  Scenario: Error state when no active session
    Given I have no active session
    When I type "/anchors"
    Then a status message displays "Start a session first to view anchor points"
    And the anchor view does not open

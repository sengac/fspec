@done
@navigation
@TUI-056 @tui @dialog @tui-component
Feature: Interactive anchor point viewer with conversation navigation

  """
  Must reuse VirtualList component for anchor display and follow existing keyboard navigation patterns
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Anchor viewer must display anchor type, weight, turn number, and timestamp for each anchor point
  #   2. Dialog interface must be similar to select mode with arrow key navigation and Enter to view details
  #   3. ONLY one command: '/anchors' - no session IDs, no options, no other access methods
  #
  # EXAMPLES:
  #   1. User runs 'fspec anchors' and sees modal with 3 anchor points: ErrorResolution (0.9), TaskCompletion (0.8), UserCheckpoint (0.7)
  #   2. User navigates with arrow keys to select TaskCompletion anchor, presses Enter, and sees turn details showing file modifications and test results
  #   3. User types '/anchors' and sees modal dialog showing anchor points from current session only
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the anchor viewer show all anchor types or allow filtering by type (ErrorResolution, TaskCompletion, etc.)?
  #   A: Show all anchor types by default with visual indicators (icons/emojis) to distinguish them. Advanced filtering can be added later if needed.
  #
  #   Q: When viewing turn details, what specific information should be displayed (tool calls, file changes, timestamps, full conversation context)?
  #   A: Show tool calls, file modifications, success/failure status, and brief context. Keep it concise - full conversation context would be too overwhelming.
  #
  #   Q: Should there be keyboard shortcuts to jump to anchor types (e.g., E for ErrorResolution, T for TaskCompletion) for faster navigation?
  #   A: Yes - use E for ErrorResolution, T for TaskCompletion, F for FeatureMilestone, U for UserCheckpoint to jump quickly between anchor types.
  #
  # ========================================

  Background: User Story
    As a developer using fspec
    I want to view and navigate anchor points in conversation sessions
    So that I can understand what moments the AI considered significant and debug context compaction decisions

  Scenario: Display anchor points in modal dialog
    Given I have a session with 3 anchor points: ErrorResolution (0.9), TaskCompletion (0.8), UserCheckpoint (0.7)
    When I run the command "/anchors"
    Then I should see a modal dialog displaying all anchor points
    And each anchor should show type, weight, turn number, and timestamp
    And visual indicators should distinguish anchor types

  Scenario: Navigate and view anchor turn details
    Given I have anchor points displayed in the modal dialog
    When I navigate with arrow keys to select the TaskCompletion anchor
    And I press Enter
    Then I should see turn details showing file modifications and test results
    And the details should include tool calls and success/failure status
    And the context should be concise, not overwhelming

  Scenario: Access anchors with simple command only
    Given I am in the fspec interface
    When I type "/anchors"
    Then I should see the modal dialog with anchor points from current session only
    And there should be no options for session IDs or other parameters
    And the command should work with this syntax only

  Scenario: Use keyboard shortcuts to jump between anchor types
    Given I have multiple anchor types displayed in the viewer
    When I press "E"
    Then I should jump to the first ErrorResolution anchor
    When I press "T"
    Then I should jump to the first TaskCompletion anchor
    When I press "F" 
    Then I should jump to the first FeatureMilestone anchor
    When I press "U"
    Then I should jump to the first UserCheckpoint anchor
@done
@session-management
@keyboard-navigation
@tui
@TUI-049
Feature: Shift+Arrow Session Switching

  """
  Architecture notes:
  - Key detection: MultiLineInput.tsx using escape sequences [1;2C (Shift+Right) and [1;2D (Shift+Left)
  - Handlers: AgentView.tsx handleSessionNext/handleSessionPrev callbacks
  - Session source: sessionManagerList() from Rust only (not merged with persisted-only sessions)
  - Navigation: List model - Right=index+1 (older), Left=index-1 (newer), with wrap-around
  - NAPI bindings: sessionDetach(oldId), sessionAttach(newId, callback), sessionGetMergedOutput(newId)
  - Input persistence: Store input text per-session in Rust (similar to debug mode), restore on reattach
  - Conversation loading: processChunksToConversation for restoring messages on attach
  - Pattern: Follows Shift+Up/Down history navigation (NAPI-006 in MultiLineInput.tsx:122-142)
  - Edge case: If current session not in background manager, treat as index -1 (navigate to first/last)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Shift+Right arrow switches to the next session in the available sessions list (sorted by updatedAt descending)
  #   2. Shift+Left arrow switches to the previous session in the available sessions list
  #   3. Session switching only works when AgentView is active (not in resume mode, search mode, settings, or model selector)
  #   4. Navigation wraps around - Shift+Right at the last session goes to the first, Shift+Left at the first goes to the last
  #   5. Switching sessions automatically attaches to the selected background session and loads its conversation
  #   6. The previously active session is automatically detached (continues running in background) before attaching to the new session
  #   7. Session switching requires at least 2 sessions to exist (no action if only 1 or 0 sessions)
  #   8. Key detection uses escape sequences [1;2C (Shift+Right) and [1;2D (Shift+Left) plus Ink key.shift detection
  #
  # EXAMPLES:
  #   1. User has 3 sessions (A, B, C), currently on session A. Pressing Shift+Right switches to session B, detaches A, and loads B's conversation.
  #   2. User is on the last session (C). Pressing Shift+Right wraps around to the first session (A).
  #   3. User is on the first session (A). Pressing Shift+Left wraps around to the last session (C).
  #   4. User has only 1 session. Pressing Shift+Left or Shift+Right does nothing (no session to switch to).
  #   5. User is in resume mode (modal open). Pressing Shift+Left or Shift+Right does nothing (keys only work in main AgentView).
  #   6. User switches from session A (running agent task) to session B. Session A continues executing in background, user sees session B's conversation immediately.
  #
  # ========================================

  Background: User Story
    As a developer using fspec TUI
    I want to quickly switch between active background sessions using Shift+Left/Right arrow keys
    So that I can navigate sessions without interrupting my workflow or opening the resume modal

  Scenario: Switch to next session with Shift+Right
    Given I have 3 background sessions A, B, and C
    And I am currently attached to session A
    When I press Shift+Right arrow
    Then session A should be detached
    And I should be attached to session B
    And I should see session B's conversation

  Scenario: Wrap around from last to first session with Shift+Right
    Given I have 3 background sessions A, B, and C
    And I am currently attached to session C (the last session)
    When I press Shift+Right arrow
    Then I should be attached to session A (the first session)

  Scenario: Wrap around from first to last session with Shift+Left
    Given I have 3 background sessions A, B, and C
    And I am currently attached to session A (the first session)
    When I press Shift+Left arrow
    Then I should be attached to session C (the last session)

  Scenario: No action with only one session
    Given I have only 1 background session
    When I press Shift+Right arrow
    Then nothing should happen
    And I should remain on the same session

  Scenario: No action when in resume mode
    Given I have multiple background sessions
    And I am in resume mode (session selection modal is open)
    When I press Shift+Right arrow
    Then nothing should happen
    And the resume modal should remain open

  Scenario: Running session continues in background after switch
    Given I have 2 background sessions A and B
    And session A is currently running an agent task
    And I am attached to session A
    When I press Shift+Right arrow
    Then I should be attached to session B
    And session A should continue executing in background
    And I should see session B's conversation immediately


  Scenario: Switch to previous session with Shift+Left
    Given I have 3 background sessions A, B, and C
    And I am currently attached to session B
    When I press Shift+Left arrow
    Then session B should be detached
    And I should be attached to session A
    And I should see session A's conversation


  Scenario: Input text preserved when switching sessions
    Given I have 2 background sessions A and B
    And I am attached to session A
    And I have typed 'hello world' in the input field
    When I press Shift+Right arrow to switch to session B
    And I press Shift+Left arrow to return to session A
    Then I should see 'hello world' in the input field


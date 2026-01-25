@navigation
@tui
@done
@VIEWNV-001
Feature: Unified Shift+Arrow Navigation Across BoardView, AgentView, and SplitPaneView
  """
  Architecture notes:
  - Create new file src/tui/utils/sessionNavigation.ts for pure navigation logic (buildNavigationList, getNextTarget, getPrevTarget)
  - Create new file src/tui/hooks/useSessionNavigation.ts - React hook wrapping navigation logic with callbacks for view switching
  - Create new file src/components/CreateSessionDialog.tsx - Confirmation dialog reusing Dialog base component
  - Modify BoardView.tsx - Add Shift+Right handler to navigate to first session or show create dialog
  - Modify AgentView.tsx - Refactor switchToSession to use shared navigation logic, add create session dialog trigger at right edge
  - Modify SplitSessionView.tsx - Add Shift+Left/Right handlers for watcher navigation (currently only has regular Left/Right for pane switching)
  - Dependencies: Uses @sengac/codelet-napi APIs (sessionManagerList, sessionGetParent, sessionGetWatchers, sessionAttach, sessionDetach)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Shift+Left from BoardView does nothing (already at leftmost position)
  #   2. Shift+Right from BoardView navigates to the first session, or shows create session dialog if no sessions exist
  #   3. Shift+Left from the first session returns to BoardView (no wrap-around)
  #   4. Shift+Right from a session with watchers navigates to its first watcher
  #   5. Shift+Right from a session without watchers navigates to the next session, or shows create session dialog if at the last session
  #   6. Shift+Left from the first watcher of a session navigates to the parent session
  #   7. Shift+Right from a watcher navigates to the next sibling watcher, or to the next session if no more siblings
  #   8. Shift+Left from a watcher navigates to the previous sibling watcher, or to the parent session if at the first sibling
  #   9. Sessions are ordered by creation time (oldest first) in the navigation list
  #   10. Create session dialog only creates regular sessions, not watchers
  #   11. The /parent command continues to work alongside Shift+Left navigation
  #   12. View mode (split vs regular AgentView) is automatically determined by whether currentSessionId has a parent
  #
  # EXAMPLES:
  #   1. From BoardView with no sessions: Shift+Right shows create session dialog
  #   2. From BoardView with sessions: Shift+Right navigates to Session A (oldest)
  #   3. From Session A (first): Shift+Left returns to BoardView
  #   4. From Session A with watchers W1, W2: Shift+Right navigates to W1
  #   5. From W1 (first watcher): Shift+Left returns to parent Session A
  #   6. From W1: Shift+Right navigates to sibling W2
  #   7. From W2 (last watcher of Session A), with Session B next: Shift+Right navigates to Session B
  #   8. From last session (no watchers) at right edge: Shift+Right shows create session dialog
  #   9. From last watcher of last session at right edge: Shift+Right shows create session dialog
  #   10. User confirms create session dialog: new unattached session is created and user navigates to it
  #   11. User cancels create session dialog: stays at current position
  #
  # QUESTIONS (ANSWERED):
  #   Q: When at the first session and pressing Shift+Left, should it go back to BoardView? Or wrap to the last session?
  #   A: Shift+Left from the first session should go back to BoardView (exit sessions entirely, no wrap-around)
  #
  #   Q: Should watchers be included in the 'create new session' flow at the right edge, or only regular (non-watcher) sessions?
  #   A: Only regular (non-watcher) sessions should be created via the create session prompt. Watchers are created through a separate workflow.
  #
  #   Q: What should be the ordering of sessions in the navigation list - creation time (oldest first), last updated (most recent first), or alphabetical by name?
  #   A: Sessions should be ordered by creation time (oldest first) in the navigation list.
  #
  #   Q: When navigating away from a watcher in SplitPaneView (e.g., Shift+Right to next session), should the split view close and show regular AgentView, or should the split view be preserved somehow?
  #   A: The split view automatically closes when switching to a non-watcher session. AgentView uses a useEffect that detects if currentSessionId has a parent - if not, it sets isWatcherSessionView=false. So navigation just changes currentSessionId and the view mode updates automatically (same pattern as /parent command).
  #
  #   Q: Should the existing /parent command continue to work alongside the new Shift+Left navigation for watcher sessions?
  #   A: Yes, keep both. The /parent command should continue to work alongside Shift+Left navigation. They serve the same purpose but /parent is explicit while Shift+Left is part of the broader navigation flow.
  #
  # ========================================
  Background: User Story
    As a developer using the fspec TUI
    I want to navigate between BoardView, sessions, and watcher sessions using Shift+Left/Right arrows
    So that I can quickly move through my workflow without switching to mouse or typing commands

  # ===========================================
  # BoardView Navigation Scenarios
  # ===========================================
  @BoardView
  Scenario: Shift+Right from BoardView with no sessions shows create session dialog
    Given I am viewing the BoardView
    And no sessions exist
    When I press Shift+Right
    Then I should see a create session dialog
    And the dialog should ask "Start New Agent?"

  @BoardView
  Scenario: Shift+Right from BoardView with sessions navigates to first session
    Given I am viewing the BoardView
    And the following sessions exist in creation order:
      | name      | created_at |
      | Session A | 2024-01-01 |
      | Session B | 2024-01-02 |
      | Session C | 2024-01-03 |
    When I press Shift+Right
    Then I should be viewing "Session A" in AgentView
    And I should see Session A's conversation

  @BoardView
  Scenario: Shift+Left from BoardView does nothing
    Given I am viewing the BoardView
    When I press Shift+Left
    Then I should remain on the BoardView

  # ===========================================
  # Session Navigation Scenarios
  # ===========================================
  @AgentView
  Scenario: Shift+Left from first session returns to BoardView
    Given I am viewing "Session A" in AgentView
    And "Session A" is the first session (oldest)
    When I press Shift+Left
    Then I should be viewing the BoardView

  @AgentView
  Scenario: Shift+Right from session with watchers navigates to first watcher
    Given I am viewing "Session A" in AgentView
    And "Session A" has the following watchers:
      | name |
      | W1   |
      | W2   |
    When I press Shift+Right
    Then I should be viewing watcher "W1" in SplitSessionView
    And the left pane should show Session A's conversation
    And the right pane should show W1's conversation

  @AgentView
  Scenario: Shift+Right from session without watchers navigates to next session
    Given I am viewing "Session A" in AgentView
    And "Session A" has no watchers
    And the following sessions exist in creation order:
      | name      |
      | Session A |
      | Session B |
    When I press Shift+Right
    Then I should be viewing "Session B" in AgentView

  @AgentView
  Scenario: Shift+Right from last session without watchers shows create session dialog
    Given I am viewing "Session C" in AgentView
    And "Session C" is the last session
    And "Session C" has no watchers
    When I press Shift+Right
    Then I should see a create session dialog

  # ===========================================
  # Watcher Navigation Scenarios
  # ===========================================
  @SplitSessionView
  Scenario: Shift+Left from first watcher returns to parent session
    Given I am viewing watcher "W1" in SplitSessionView
    And "W1" is the first watcher of "Session A"
    When I press Shift+Left
    Then I should be viewing "Session A" in AgentView
    And the split view should close

  @SplitSessionView
  Scenario: Shift+Right from watcher navigates to next sibling watcher
    Given I am viewing watcher "W1" in SplitSessionView
    And "Session A" has the following watchers:
      | name |
      | W1   |
      | W2   |
    When I press Shift+Right
    Then I should be viewing watcher "W2" in SplitSessionView

  @SplitSessionView
  Scenario: Shift+Left from watcher navigates to previous sibling watcher
    Given I am viewing watcher "W2" in SplitSessionView
    And "Session A" has the following watchers:
      | name |
      | W1   |
      | W2   |
    When I press Shift+Left
    Then I should be viewing watcher "W1" in SplitSessionView

  @SplitSessionView
  Scenario: Shift+Right from last watcher navigates to next session
    Given I am viewing watcher "W2" in SplitSessionView
    And "W2" is the last watcher of "Session A"
    And the following sessions exist in creation order:
      | name      |
      | Session A |
      | Session B |
    When I press Shift+Right
    Then I should be viewing "Session B" in AgentView

  @SplitSessionView
  Scenario: Shift+Right from last watcher of last session shows create session dialog
    Given I am viewing watcher "W2" in SplitSessionView
    And "W2" is the last watcher of "Session C"
    And "Session C" is the last session
    When I press Shift+Right
    Then I should see a create session dialog

  # ===========================================
  # Create Session Dialog Scenarios
  # ===========================================
  @CreateSessionDialog
  Scenario: Confirming create session dialog creates new unattached session
    Given I see a create session dialog
    When I confirm the dialog
    Then a new session should be created
    And the new session should not be attached to any work unit
    And I should be viewing the new session in AgentView

  @CreateSessionDialog
  Scenario: Canceling create session dialog stays at current position
    Given I am viewing "Session C" in AgentView
    And I see a create session dialog
    When I cancel the dialog
    Then the dialog should close
    And I should remain viewing "Session C" in AgentView

  # ===========================================
  # Backward Compatibility Scenario
  # ===========================================
  @BackwardCompatibility
  Scenario: /parent command still works alongside Shift+Left navigation
    Given I am viewing watcher "W1" in SplitSessionView
    And "W1" is a watcher of "Session A"
    When I type "/parent" and press Enter
    Then I should be viewing "Session A" in AgentView
    And the split view should close

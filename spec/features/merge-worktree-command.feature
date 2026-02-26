@GIT-036 @wip
Feature: Merge worktree slash command with auto-close session workflow

  """
  Handler location: AgentView.tsx handleSubmit (post-session code path). Pattern: check userMessage === '/merge-worktree', clear input, check isIsolated from sessionStore, call inspectSessionChanges to detect clean state, then mergeSessionChanges. Use setConversation() with type:'status' for output. Close session via cleanupCurrentSessionHandler() + destroySession() + onExit(). Remove /sessions handler, SessionManagementPanel component and tests, and 'sessions' from SLASH_COMMANDS registry.
  """

  Background: User Story
    As a developer using an isolated session
    I want to merge my worktree changes back to the main project and close the session in one step
    So that I have a clear, intent-driven workflow to finish my isolated work

  # --- /merge-worktree command ---

  Scenario: Successful merge closes session and returns to board
    Given I am in an active isolated session with modified files
    When I type "/merge-worktree"
    Then the worktree changes should be applied to the main worktree
    And I should see a merge summary in the chat showing files modified, added, and deleted
    And the session should be destroyed
    And I should be returned to the board view

  Scenario: Merge worktree in non-isolated session shows error
    Given I am in an active session that is not isolated
    When I type "/merge-worktree"
    Then I should see an error message "This command is only available in isolated sessions"
    And the session should remain active

  Scenario: Merge worktree with no changes shows nothing to merge
    Given I am in an active isolated session with no modified files
    When I type "/merge-worktree"
    Then I should see a message "Nothing to merge"
    And the session should remain active

  Scenario: Merge worktree with conflicts keeps session open
    Given I am in an active isolated session with modified files
    And the main worktree has conflicting changes
    When I type "/merge-worktree"
    Then I should see conflict details in the chat
    And the session should remain active for conflict resolution

  # --- /sessions removal ---

  Scenario: /sessions command is removed
    Given the slash command registry is loaded
    Then the "sessions" command should not be in the registry

  Scenario: /merge-worktree command is registered
    Given the slash command registry is loaded
    Then the "merge-worktree" command should be in the registry
    And it should require an active session

  Scenario: SessionManagementPanel component is removed
    Given the codebase after this change
    Then the file "src/tui/components/SessionManagementPanel.tsx" should not exist
    And AgentView should not import SessionManagementPanel
    And AgentView should not contain showSessionManagementPanel state

@session-management
@tui-component
@GIT-029
Feature: Session Management Panel
  """
  TUI components for isolated session management:
  - CreateSessionDialog with Isolated toggle option
  - SessionManagementPanel for viewing and managing completed sessions
  """

  Background: User Story
    As a developer
    I want to create and manage isolated sessions from the TUI
    So that I can safely run AI agents in git worktrees and review/merge/discard their changes

  # ========================================
  # Part A: Session Creation
  # ========================================
  @tui
  Scenario: Create session with isolated toggle disabled (default)
    Given the TUI session creation dialog is open
    And the "Isolated" toggle is OFF
    When I submit the session creation form
    Then sessionManagerCreateWithId should be called
    And the session should use the project root as working directory

  @tui
  Scenario: Create session with isolated toggle enabled
    Given the TUI session creation dialog is open
    And the "Isolated" toggle is ON
    When I submit the session creation form
    Then sessionManagerCreateIsolated should be called
    And a worktree should be created at ".fspec/worktrees/<session-id>/"
    And the session info should display the worktree path

  # ========================================
  # Part B: Session Management Panel
  # ========================================
  @tui
  Scenario: View Session Management Panel with pending sessions
    Given there are completed isolated sessions with worktrees
    When I open the Session Management Panel
    Then I should see a list of sessions
    And each session should display its status badge
    And pending_merge sessions should have a yellow badge
    And clean sessions should have a green badge
    And orphaned sessions should have a red badge
    And each session should show the files changed count

  @tui
  Scenario: Merge a pending_merge session
    Given the Session Management Panel is open
    And there is a session with status "pending_merge"
    When I click the Merge button for that session
    Then a confirmation dialog should appear
    When I confirm the merge
    Then mergeSession NAPI binding should be called
    And the session changes should be applied to the main worktree
    And the session should be removed from the list

  @tui
  Scenario: Discard a pending_merge session
    Given the Session Management Panel is open
    And there is a session with status "pending_merge"
    When I click the Discard button for that session
    Then a confirmation dialog should appear
    When I confirm the discard
    Then discardSession NAPI binding should be called
    And the worktree should be removed
    And no changes should be applied to the main worktree
    And the session should be removed from the list

  @tui
  Scenario: Prune orphaned sessions
    Given the Session Management Panel is open
    And there are orphaned sessions
    When I click the "Prune Orphaned" button
    Then pruneOrphaned NAPI binding should be called
    And all orphaned sessions should be removed
    And a confirmation should show the count of pruned sessions

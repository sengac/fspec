@session-management
@codelet
@GIT-029
Feature: Isolated session NAPI bindings

  """
  NAPI bindings for isolated session management: listSessions, inspectSession,
  mergeSession, discardSession, pruneOrphaned.
  These bindings exist in codelet/napi/src/git.rs and provide the foundation
  for TUI session management.
  """

  Background: User Story
    As a developer
    I want to manage isolated sessions via NAPI bindings
    So that the TUI can list, inspect, merge, discard, and prune sessions

  @napi
  Scenario: listSessions returns sessions with derived status
    Given a git repository at the project root
    And there are session worktrees in ".fspec/worktrees/"
    When I call listSessions with active session IDs
    Then sessions with worktrees should be returned
    And each session should have a derived status
    And active sessions should have status "active"
    And sessions with changes but not active should have status "pending_merge"
    And sessions without changes and not active should have status "clean"
    And sessions without records should have status "orphaned"

  @napi
  Scenario: inspectSession returns diff without side effects
    Given a git repository at the project root
    And an isolated session worktree exists
    And the session has modified files
    When I call inspectSession for that session
    Then a SessionResult should be returned
    And the result should contain a unified diff
    And the result should contain lists of changed, added, and deleted files
    And the worktree should remain intact

  @napi
  Scenario: mergeSession applies changes and removes worktree
    Given a git repository at the project root
    And an isolated session worktree exists
    And the session has modified files
    When I call mergeSession for that session
    Then the modified files should be copied to the main worktree
    And new files should be added to the main worktree
    And deleted files should be removed from the main worktree
    And the session worktree should be removed
    And a MergeResult should be returned with file lists

  @napi
  Scenario: discardSession removes worktree without applying changes
    Given a git repository at the project root
    And an isolated session worktree exists
    And the session has modified files
    When I call discardSession for that session
    Then the worktree should be removed
    And no files should be modified in the main worktree
    And a DiscardResult should be returned with the files discarded count

  @napi
  Scenario: pruneOrphaned removes worktrees with no session records
    Given a git repository at the project root
    And there are orphaned worktrees with no session records
    When I call pruneOrphaned with active session IDs
    Then all orphaned worktrees should be removed
    And a PruneResult should be returned with the count of pruned sessions

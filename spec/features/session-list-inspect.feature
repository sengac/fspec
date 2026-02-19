@rust
@session-management
@git
@GIT-023
Feature: Session Manager List and Inspect
  """
  Implement SessionInfo and SessionFilter in codelet/git/src/session_manager.rs alongside derive_session_status from GIT-022
  list_sessions() uses list_worktrees() from GIT-014 and derive_session_status() from GIT-022
  inspect_session() wraps get_session_diff() from GIT-015 - no modification, read-only operation
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. list_sessions() returns all sessions with derived status (Active, PendingMerge, Clean, Orphaned)
  #   2. SessionInfo includes: session_id, status, base_commit, files_changed, created_at, worktree_path
  #   3. SessionFilter enum supports: All, Active, PendingMerge, Clean, Orphaned filters
  #   4. inspect_session() returns SessionResult (diff) without modifying anything
  #   5. inspect_session() for non-existent session returns WorktreeNotFound error
  #   6. list_sessions() returns empty Vec when no worktrees exist
  #
  # EXAMPLES:
  #   1. list_sessions(All) with 3 sessions → returns all 3 with correct status for each
  #   2. list_sessions(Orphaned) with 2 orphaned, 1 active → returns only the 2 orphaned
  #   3. list_sessions(PendingMerge) → returns only sessions with uncommitted changes
  #   4. list_sessions(All) with no worktrees → returns empty Vec
  #   5. inspect_session(existing_id) → returns SessionResult with files_changed, files_added, files_deleted
  #   6. inspect_session with deleted file → SessionResult.files_deleted contains the file path
  #   7. inspect_session on clean worktree → empty files_changed, files_added, files_deleted
  #   8. inspect_session(non_existent_id) → WorktreeNotFound error
  #
  # ========================================
  Background: User Story
    As a AI coding agent
    I want to list sessions and inspect their diffs
    So that review changes before deciding to merge or discard

  # ========================================
  # LIST SESSIONS SCENARIOS
  # ========================================
  Scenario: List all session worktrees with status information
    Given a repository with multiple session worktrees
    And one session is active
    And one session has pending merge status
    And one session is orphaned
    When I call list_sessions with All filter
    Then I should receive 3 SessionInfo objects
    And each SessionInfo should contain session_id, status, base_commit, files_changed, created_at, worktree_path
    And the status should be correctly derived for each session

  Scenario: List only orphaned session worktrees
    Given a repository with multiple session worktrees
    And 2 sessions are orphaned
    And 1 session is active
    When I call list_sessions with Orphaned filter
    Then I should receive 2 SessionInfo objects
    And all returned sessions should have Orphaned status

  Scenario: List sessions with pending_merge filter
    Given a repository with multiple session worktrees
    And some sessions have uncommitted changes
    When I call list_sessions with PendingMerge filter
    Then I should only receive sessions with PendingMerge status
    And sessions without changes should not be included

  Scenario: List sessions returns empty when no worktrees exist
    Given a repository with no session worktrees
    When I call list_sessions with All filter
    Then I should receive an empty Vec
    And no error should be returned

  # ========================================
  # INSPECT SESSION SCENARIOS
  # ========================================
  Scenario: Inspect session diff before merging
    Given a session worktree with modified files
    And the session has files_changed, files_added, and files_deleted
    When I call inspect_session with the session ID
    Then I should receive a SessionResult
    And the SessionResult should contain files_changed list
    And the SessionResult should contain files_added list
    And the SessionResult should contain files_deleted list
    And the worktree should not be modified

  Scenario: Inspect session shows deleted files
    Given a session worktree with a deleted file
    When I call inspect_session with the session ID
    Then the SessionResult.files_deleted should contain the deleted file path

  Scenario: Inspect clean session returns empty diff
    Given a session worktree with no changes
    When I call inspect_session with the session ID
    Then the SessionResult.files_changed should be empty
    And the SessionResult.files_added should be empty
    And the SessionResult.files_deleted should be empty

  Scenario: Inspect session fails for non-existent session
    Given a session ID that does not exist
    When I call inspect_session with the non-existent session ID
    Then I should receive a WorktreeNotFound error

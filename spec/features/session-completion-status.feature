@done
@rust
@session-management
@git
@GIT-022
Feature: Session Completion and Status Derivation
  """
  Add SessionStatus enum to codelet/git/src/session_manager.rs with Active, PendingMerge, Clean, Orphaned variants
  Add derive_session_status(repo_path, session_id, active_sessions) to codelet/git/src/session_manager.rs
  Modify BackgroundSession::complete() in codelet/napi/src/session_manager.rs to NOT cleanup worktree
  Session manifest at ~/.fspec/sessions/<session-id>.json stores session metadata for orphan detection
  Uses get_session_diff() from GIT-015 to determine if worktree has changes
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Session completion does NOT cleanup worktree - leaves it for user review
  #   2. SessionStatus enum has 4 states: Active, PendingMerge, Clean, Orphaned
  #   3. Status derivation checks BackgroundSession active map FIRST before checking worktree state
  #   4. Active sessions always return Active status regardless of worktree changes
  #   5. PendingMerge status: worktree exists, not active, HAS uncommitted changes
  #   6. Clean status: worktree exists, not active, NO uncommitted changes
  #   7. Orphaned status: worktree exists but no session record (manifest missing or terminated)
  #   8. Session manifest stored at ~/.fspec/sessions/<session-id>.json
  #   9. Manifest tracks: session_id, project_root, worktree_path, base_commit, created_at, completed_at, terminated
  #
  # EXAMPLES:
  #   1. Complete isolated session → worktree still exists at .fspec/worktrees/<session-id>/
  #   2. Complete session without changes → status is Clean
  #   3. Complete session with uncommitted changes → status is PendingMerge
  #   4. Query status of active session → returns Active (not PendingMerge even if changes exist)
  #   5. Query status of worktree with no manifest → returns Orphaned
  #   6. Query status of worktree with terminated=true in manifest → returns Orphaned
  #   7. Session completion updates manifest with completed_at timestamp
  #
  # ========================================
  Background: User Story
    As a developer
    I want to have session status derived at query time and worktrees preserved on completion
    So that review changes before merging and always see accurate session state

  # Example 1: Session completion preserves worktree for review
  Scenario: Isolated session completion leaves worktree for review
    Given I have a git repository
    And I create an isolated session "abc123"
    And the session has a worktree at ".fspec/worktrees/abc123"
    When I complete the session
    Then the worktree at ".fspec/worktrees/abc123" should still exist
    And the worktree should not be automatically cleaned up

  # Example 2: Completed session without changes has Clean status
  Scenario: Session without changes transitions to Clean status on completion
    Given I have a git repository
    And I create an isolated session "abc123"
    And the session worktree has no uncommitted changes
    When I complete the session
    And I derive the session status for "abc123"
    Then the status should be "Clean"

  # Example 3: Completed session with changes has PendingMerge status
  Scenario: Session with changes transitions to PendingMerge status on completion
    Given I have a git repository
    And I create an isolated session "abc123"
    And the session worktree has uncommitted changes
    When I complete the session
    And I derive the session status for "abc123"
    Then the status should be "PendingMerge"

  # Example 4: Active sessions always show Active status
  Scenario: Active session returns Active status regardless of changes
    Given I have a git repository
    And I create an isolated session "abc123"
    And the session is still active
    And the session worktree has uncommitted changes
    When I derive the session status for "abc123"
    Then the status should be "Active"
    And the status should not be "PendingMerge"

  # Example 5: Worktree without manifest is Orphaned
  Scenario: Worktree with no manifest returns Orphaned status
    Given I have a git repository
    And a worktree exists at ".fspec/worktrees/orphan123"
    And no session manifest exists for "orphan123"
    When I derive the session status for "orphan123"
    Then the status should be "Orphaned"

  # Example 6: Terminated session is Orphaned
  Scenario: Worktree with terminated manifest returns Orphaned status
    Given I have a git repository
    And a worktree exists at ".fspec/worktrees/terminated123"
    And a session manifest exists for "terminated123" with terminated=true
    When I derive the session status for "terminated123"
    Then the status should be "Orphaned"

  # Example 7: Session manifest updated on completion
  Scenario: Session completion updates manifest with completed_at timestamp
    Given I have a git repository
    And I create an isolated session "abc123"
    And a session manifest exists for "abc123" without completed_at
    When I complete the session
    Then the session manifest for "abc123" should have a completed_at timestamp

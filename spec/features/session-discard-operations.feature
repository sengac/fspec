@git
@session-management
@rust
@GIT-025
Feature: Session Manager Discard Operations
  """
  Implement discard_session() and DiscardResult in rust/git/src/session_status.rs alongside merge_session()
  discard_session() uses abort_session() from session_result.rs (GIT-015) to remove worktree
  discard_session() uses delete_manifest() from session_status.rs to clean up manifest
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. discard_session() removes worktree without applying any changes
  #   2. discard_session() uses abort_session() primitive from GIT-015
  #   3. discard_session() deletes session manifest from ~/.fspec/git-sessions/
  #   4. DiscardResult includes: session_id, files_discarded count, previous_status
  #   5. discard_session() for non-existent session returns WorktreeNotFound error
  #   6. discard_session() works for any session status (Active, PendingMerge, Clean, Orphaned)
  #
  # EXAMPLES:
  #   1. discard_session(session_with_changes) → worktree removed, main unchanged, DiscardResult.files_discarded > 0
  #   2. discard_session(clean_session) → worktree removed, DiscardResult.files_discarded = 0
  #   3. discard_session(non_existent) → WorktreeNotFound error
  #   4. discard_session(orphaned_session) → worktree removed, DiscardResult.previous_status = Orphaned
  #   5. discard_session removes manifest file from ~/.fspec/git-sessions/
  #
  # ========================================
  Background: User Story
    As a AI coding agent
    I want to discard session worktrees without applying changes
    So that clean up sessions I decide not to use

  # ========================================
  # DISCARD SESSION SCENARIOS
  # ========================================
  Scenario: Discard session without applying changes
    Given a git repository with an initial commit
    And a session worktree with a modified file "src/main.rs"
    When I call discard_session with the session ID
    Then the session worktree should be removed
    And the main worktree should NOT contain the modified content
    And the DiscardResult should contain files_discarded greater than 0

  Scenario: Discard clean session without confirmation
    Given a git repository with an initial commit
    And a session worktree with no changes
    When I call discard_session with the session ID
    Then the session worktree should be removed
    And the DiscardResult should contain files_discarded equal to 0

  Scenario: Discard session fails for non-existent session
    Given a git repository with an initial commit
    And a session ID that does not exist
    When I call discard_session with the non-existent session ID
    Then I should receive a WorktreeNotFound error

  Scenario: Discard orphaned session removes worktree
    Given a git repository with an initial commit
    And an orphaned session worktree
    When I call discard_session with the session ID
    Then the session worktree should be removed
    And the DiscardResult should have previous_status equal to Orphaned

  Scenario: Discard session cleans up manifest
    Given a git repository with an initial commit
    And a session worktree with a manifest in ~/.fspec/git-sessions/
    When I call discard_session with the session ID
    Then the session worktree should be removed
    And the session manifest should be deleted from ~/.fspec/git-sessions/

@rust
@session
@session-management
@git
@GIT-026
Feature: Orphan Detection and Pruning
  """
  Implement in codelet/git/src/session_status.rs alongside existing session management code
  Uses existing list_worktrees(), remove_worktree(), and delete_manifest() functions
  is_orphaned() simplifies status checking by focusing only on orphan detection (subset of derive_session_status logic)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A session is orphaned if it is NOT in the active sessions set AND (manifest doesn't exist OR manifest.terminated == true)
  #   2. Active sessions are NEVER orphaned regardless of manifest state
  #   3. prune_orphaned() removes all orphaned worktrees and their associated manifests
  #   4. prune_orphaned() returns PruneResult with count of pruned worktrees and list of pruned session IDs
  #   5. When no orphaned worktrees exist, prune returns count=0 and empty list
  #
  # EXAMPLES:
  #   1. Session with worktree but missing manifest → is_orphaned returns true
  #   2. Session with worktree and manifest.terminated=true → is_orphaned returns true
  #   3. Session in active_sessions set with missing manifest → is_orphaned returns false (active overrides)
  #   4. Session with worktree and valid non-terminated manifest → is_orphaned returns false
  #   5. prune_orphaned with 3 orphaned worktrees → returns PruneResult{count: 3, pruned: [id1, id2, id3]}
  #   6. prune_orphaned with no orphaned worktrees → returns PruneResult{count: 0, pruned: []}
  #   7. prune_orphaned removes both worktree directory and manifest file for each pruned session
  #
  # ========================================
  Background: User Story
    As a developer using fspec's session isolation
    I want to detect and prune orphaned worktrees
    So that I can clean up disk space and remove stale session worktrees

  # ===========================================
  # ORPHAN DETECTION SCENARIOS
  # ===========================================
  @orphan-detection
  Scenario: Detect orphaned worktree when session manifest is missing
    Given a git repository with an isolated session worktree "session-1"
    And no session manifest exists for "session-1"
    And "session-1" is not in the active sessions set
    When I check if "session-1" is orphaned
    Then the session should be detected as orphaned

  @orphan-detection
  Scenario: Detect orphaned worktree when session manifest is terminated
    Given a git repository with an isolated session worktree "session-2"
    And a session manifest exists for "session-2" with terminated flag set to true
    And "session-2" is not in the active sessions set
    When I check if "session-2" is orphaned
    Then the session should be detected as orphaned

  @orphan-detection
  Scenario: Active session with missing manifest is not orphaned
    Given a git repository with an isolated session worktree "session-3"
    And no session manifest exists for "session-3"
    And "session-3" is in the active sessions set
    When I check if "session-3" is orphaned
    Then the session should NOT be detected as orphaned

  @orphan-detection
  Scenario: Session with valid non-terminated manifest is not orphaned
    Given a git repository with an isolated session worktree "session-4"
    And a session manifest exists for "session-4" with terminated flag set to false
    And "session-4" is not in the active sessions set
    When I check if "session-4" is orphaned
    Then the session should NOT be detected as orphaned

  # ===========================================
  # PRUNE SCENARIOS
  # ===========================================
  @prune
  Scenario: Prune all orphaned worktrees
    Given a git repository with 3 isolated session worktrees
    And all 3 sessions have no manifest files
    And none of the sessions are active
    When I prune orphaned worktrees
    Then the result should indicate 3 worktrees were pruned
    And the result should contain all 3 session IDs
    And all worktree directories should be removed
    And all session manifest files should be cleaned up

  @prune
  Scenario: Prune returns zero when no orphaned worktrees exist
    Given a git repository with 2 isolated session worktrees
    And all sessions have valid non-terminated manifest files
    When I prune orphaned worktrees
    Then the result should indicate 0 worktrees were pruned
    And the result should contain an empty list of pruned session IDs
    And all worktree directories should still exist

  @prune
  Scenario: Prune returns list of pruned session IDs
    Given a git repository with 2 orphaned session worktrees "orphan-1" and "orphan-2"
    And 1 active session worktree "active-1"
    When I prune orphaned worktrees
    Then the result should indicate 2 worktrees were pruned
    And the result should contain session IDs "orphan-1" and "orphan-2"
    And the result should NOT contain session ID "active-1"

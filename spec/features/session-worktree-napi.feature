@napi
@GIT-027
Feature: Session Worktree NAPI Bindings
  """
  NAPI bindings in codelet/napi/src/git.rs expose Rust session_status.rs functions (list_sessions, inspect_session, merge_session, discard_session, prune_orphaned) to TypeScript.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. createIsolatedSession() NAPI function accepts repo_path and CreateIsolatedSessionOptions (session_id, base_ref)
  #   2. listSessions() NAPI function accepts repo_path and optional filter string ('all', 'active', 'pending_merge', 'clean', 'orphaned')
  #   3. inspectSession() NAPI function returns SessionResultJs with diff, files_changed, files_added, files_deleted
  #   4. mergeSession() NAPI function returns MergeResultJs with files_modified, files_added, files_deleted
  #   5. mergeSession() throws NAPI error with conflict file list when main worktree has conflicting changes
  #   6. discardSession() NAPI function returns DiscardResultJs with session_id, files_discarded count
  #   7. pruneOrphaned() NAPI function returns PruneResultJs with count and list of pruned session IDs
  #
  # EXAMPLES:
  #   1. createIsolatedSession(repo, {sessionId: 'feature-auth', baseRef: 'main'}) → WorktreeCreateResultJs with worktreePath, baseCommit, createdAt
  #   2. listSessions(repo, 'all') → returns array of SessionInfoJs with session_id, status, base_commit, files_changed
  #   3. listSessions(repo, 'pending_merge') → returns only sessions with uncommitted changes
  #   4. inspectSession(repo, 'feature-auth') → SessionResultJs with diff string and file lists
  #   5. mergeSession(repo, 'feature-auth') → MergeResultJs with files_modified, files_added, files_deleted arrays
  #   6. mergeSession(repo, 'conflict-session') → throws Error with 'Conflict: file1.txt, file2.txt' in message
  #   7. discardSession(repo, 'feature-auth') → DiscardResultJs with session_id and files_discarded count
  #   8. pruneOrphaned(repo) → PruneResultJs with count: 3 and pruned: ['id1', 'id2', 'id3']
  #
  # ========================================
  Background: User Story
    As a TypeScript developer
    I want to call session worktree operations from TypeScript
    So that build TUI and command features that manage isolated sessions

  Scenario: Create isolated session with worktree
    Given a git repository at "/project"
    When I call createIsolatedSession with sessionId "feature-auth" and baseRef "main"
    Then the result contains worktreePath, baseCommit, and createdAt fields
    And a worktree directory exists at ".fspec/worktrees/feature-auth/"

  Scenario: List all session worktrees with status
    Given a git repository with 3 existing session worktrees
    When I call listSessions with filter "all"
    Then the result is an array of SessionInfoJs with 3 entries
    And each entry contains sessionId, status, baseCommit, filesChanged, createdAt, and worktreePath

  Scenario: List sessions with pending merge filter
    Given a git repository with 2 clean sessions and 1 session with uncommitted changes
    When I call listSessions with filter "pending_merge"
    Then the result contains only the 1 session with uncommitted changes

  Scenario: Inspect session diff before merging
    Given an isolated session "feature-auth" with modified files
    When I call inspectSession for "feature-auth"
    Then the result contains diff string, filesChanged, filesAdded, and filesDeleted arrays
    And the session worktree remains unchanged

  Scenario: Merge session changes to main worktree
    Given an isolated session "feature-auth" with modified and added files
    When I call mergeSession for "feature-auth"
    Then the result contains filesModified, filesAdded, and filesDeleted arrays
    And the session worktree is removed
    And the changes appear in the main worktree

  Scenario: Merge session returns conflict error
    Given an isolated session "conflict-session" with modified files
    And the same files were modified in the main worktree since base commit
    When I call mergeSession for "conflict-session"
    Then the function throws an Error with "Conflict" in the message
    And the error message contains the list of conflicting file paths
    And the session worktree remains intact

  Scenario: Discard session without applying changes
    Given an isolated session "feature-auth" with 3 modified files
    When I call discardSession for "feature-auth"
    Then the result contains sessionId and filesDiscarded count of 3
    And the session worktree is removed
    And the main worktree is unchanged

  Scenario: Prune orphaned worktrees
    Given 3 orphaned worktrees exist with no valid session manifests
    When I call pruneOrphaned
    Then the result contains count of 3 and an array of pruned session IDs
    And the 3 orphaned worktree directories are removed

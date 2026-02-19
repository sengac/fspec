@done
@session-merge
@git
@GIT-024
Feature: Session Manager Merge Operations
  """
  Implement merge_session() in session_status.rs alongside list_sessions() and inspect_session() from GIT-023
  merge_session() wraps apply_session_changes() from session_result.rs (GIT-015) - captures diff before applying, returns MergeResult
  MergeResult struct contains: session_id, files_modified, files_added, files_deleted
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. merge_session() returns MergeResult with files_modified, files_added, files_deleted
  #   2. Worktree is removed after successful merge
  #   3. ConflictError returned when files modified in both session and main since base_commit
  #   4. ConflictError returned when session adds file that exists in main with different content
  #   5. Worktree remains intact after conflict (user can resolve and retry)
  #   6. Clean sessions (no changes) can be merged - just removes worktree
  #   7. Multiple sessions can be merged in user-chosen order
  #
  # EXAMPLES:
  #   1. Session modifies file.txt → merge_session() → file.txt updated in main, worktree removed, MergeResult.files_modified = ["file.txt"]
  #   2. Session adds new.txt → merge_session() → new.txt created in main, MergeResult.files_added = ["new.txt"]
  #   3. Session deletes old.txt → merge_session() → old.txt removed from main, MergeResult.files_deleted = ["old.txt"]
  #   4. Both session and main modify same file → merge_session() → ConflictError with file list, worktree intact
  #   5. Session adds file that exists in main with different content → merge_session() → ConflictError
  #   6. 3 pending sessions A, B, C → user merges B, then A, then C → all succeed in user order
  #   7. Clean session (no changes) → merge_session() → worktree removed, MergeResult with empty file lists
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want to merge session changes to the main worktree
    So that apply isolated work while detecting conflicts

  # ===========================================
  # Successful Merge Scenarios
  # ===========================================
  Scenario: Merge session changes to main worktree
    Given a git repository with an initial commit
    And a session worktree with a modified file "src/main.rs"
    When I call merge_session with the session ID
    Then the modified file should be updated in the main worktree
    And the session worktree should be removed
    And the MergeResult should contain "src/main.rs" in files_modified

  Scenario: Merge session applies added files
    Given a git repository with an initial commit
    And a session worktree with a new file "src/new.rs"
    When I call merge_session with the session ID
    Then the new file should exist in the main worktree
    And the session worktree should be removed
    And the MergeResult should contain "src/new.rs" in files_added

  Scenario: Merge session applies deleted files
    Given a git repository with an initial commit containing "src/old.rs"
    And a session worktree where "src/old.rs" has been deleted
    When I call merge_session with the session ID
    Then "src/old.rs" should not exist in the main worktree
    And the session worktree should be removed
    And the MergeResult should contain "src/old.rs" in files_deleted

  # ===========================================
  # Conflict Detection Scenarios
  # ===========================================
  Scenario: Merge session fails when main has conflicting changes
    Given a git repository with an initial commit containing "src/config.rs"
    And a session worktree where "src/config.rs" has been modified
    And "src/config.rs" has also been modified in the main worktree
    When I call merge_session with the session ID
    Then a ConflictError should be returned
    And the ConflictError should list "src/config.rs" as a conflicting file
    And the session worktree should still exist

  Scenario: Merge session fails when added file conflicts with main
    Given a git repository with an initial commit
    And a session worktree with a new file "src/feature.rs" containing "session content"
    And the main worktree also has "src/feature.rs" with different content
    When I call merge_session with the session ID
    Then a ConflictError should be returned
    And the ConflictError should list "src/feature.rs" as a conflicting file
    And the session worktree should still exist

  # ===========================================
  # Multiple Sessions and Clean Session
  # ===========================================
  Scenario: Merge multiple pending sessions in chosen order
    Given a git repository with an initial commit
    And three session worktrees "session-A", "session-B", "session-C" each with different changes
    When I merge sessions in order: "session-B", "session-A", "session-C"
    Then all merges should succeed
    And all session worktrees should be removed
    And the main worktree should contain changes from all sessions

  Scenario: Merge clean session removes worktree
    Given a git repository with an initial commit
    And a session worktree with no changes
    When I call merge_session with the session ID
    Then the session worktree should be removed
    And the MergeResult should have empty files_modified
    And the MergeResult should have empty files_added
    And the MergeResult should have empty files_deleted

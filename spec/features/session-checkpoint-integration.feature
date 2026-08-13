@GIT-021
Feature: Session Checkpoint Integration
  """
  Primary implementation in rust/napi/src/session_manager.rs - add checkpoint(), restore(), list_checkpoints() methods to BackgroundSession
  Uses create_ghost_commit(), restore_ghost_commit(), list_ghost_checkpoints() from rust/git/src/ghost_commit.rs
  Add SessionError enum with NotIsolated variant for error handling
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. checkpoint() method MUST only work for isolated sessions (worktree_path is Some)
  #   2. checkpoint() calls create_ghost_commit() with session worktree path as the directory
  #   3. Checkpoints are stored at refs/fspec-checkpoints/<session-id>/<label> using session_id as work_unit_id
  #   4. restore() calls restore_ghost_commit() to return worktree to checkpoint state
  #   5. list_checkpoints() returns all checkpoint labels for the session via list_ghost_checkpoints()
  #   6. NotIsolated error MUST be returned when checkpoint operations are called on non-isolated sessions
  #
  # EXAMPLES:
  #   1. Isolated session calls checkpoint('before-refactor'), ghost commit created at refs/fspec-checkpoints/<session-id>/before-refactor
  #   2. Isolated session with staged, unstaged, and untracked files calls checkpoint(), all file states captured in ghost commit
  #   3. Isolated session calls restore('before-refactor'), worktree files restored to checkpoint state
  #   4. Two isolated sessions both create checkpoint 'baseline' - each stored at different refs under their session IDs
  #   5. Non-isolated session calls checkpoint(), returns NotIsolated error
  #   6. Isolated session calls list_checkpoints() after creating 3 checkpoints, returns all 3 checkpoint labels
  #
  # ========================================
  Background: User Story
    As a isolated AI agent session
    I want to create and restore checkpoints
    So that safely experiment with changes and rollback when needed

  @unit
  Scenario: Checkpoint creates ghost commit with session ID namespace
    Given an isolated session with worktree path
    When I call checkpoint with label "before-refactor"
    Then a ghost commit should be created
    And the ref should be stored at refs/fspec-checkpoints/<session-id>/before-refactor

  @unit
  Scenario: Checkpoint captures all worktree changes
    Given an isolated session with worktree path
    And there are staged files in the worktree
    And there are unstaged modifications in the worktree
    And there are untracked files in the worktree
    When I call checkpoint with label "full-state"
    Then all file states should be captured in the ghost commit

  @unit
  Scenario: Restore checkpoint returns worktree to checkpoint state
    Given an isolated session with worktree path
    And I have created a checkpoint named "before-refactor"
    And I have modified files after the checkpoint
    When I call restore with label "before-refactor"
    Then the worktree files should match the checkpoint state
    And files added after checkpoint should be deleted

  @unit
  Scenario: Parallel sessions have independent checkpoint namespaces
    Given two isolated sessions with different IDs
    And both sessions create checkpoint named "baseline"
    Then each checkpoint should be stored under its own session ID
    And session A should not see session B checkpoints

  @unit
  Scenario: Checkpoint fails for non-isolated session
    Given a non-isolated session without worktree path
    When I call checkpoint with label "test"
    Then a NotIsolated error should be returned

  @unit
  Scenario: List checkpoints returns all checkpoint labels
    Given an isolated session with worktree path
    And I have created checkpoints named "first", "second", "third"
    When I call list_checkpoints
    Then all three checkpoint labels should be returned

@git-integration
@session-management
@session
@GIT-019
Feature: Isolated Session Creation and effective_cwd
  """
  Primary implementation in codelet/napi/src/session_manager.rs. Add worktree_path and base_commit fields to BackgroundSession struct. Uses create_worktree() from codelet/git/src/worktree.rs (GIT-014). Worktrees stored at .fspec/worktrees/<session-id>/. effective_cwd() method returns worktree path or project root based on isolation mode.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Session creation accepts an 'isolated' parameter (defaults to false)
  #   2. When isolated=true, a worktree is created at .fspec/worktrees/<session-id>/
  #   3. Session state tracks worktree_path and base_commit for isolated sessions
  #   4. effective_cwd() returns worktree path for isolated sessions, project root for non-isolated
  #   5. Creating an isolated session fails with WorktreeExists error if worktree already exists
  #
  # EXAMPLES:
  #   1. Create session with isolated=true, worktree appears at .fspec/worktrees/abc123/
  #   2. Create session with isolated=false, no worktree directory created
  #   3. Create session without specifying isolated, behaves as isolated=false
  #   4. Call effective_cwd() on isolated session, returns /project/.fspec/worktrees/abc123
  #   5. Call effective_cwd() on non-isolated session, returns /project
  #   6. Try to create isolated session when worktree exists, get WorktreeExists error
  #
  # ========================================
  Background: User Story
    As a developer
    I want to create isolated sessions with git worktrees
    So that multiple AI agents can work in parallel without file conflicts

  Scenario: Create isolated session with worktree
    Given a git repository at "/project"
    And no worktree exists for session "abc123"
    When I create a session with id "abc123" and isolated=true
    Then a worktree should be created at ".fspec/worktrees/abc123/"
    And the session state should include worktree_path
    And the session state should include base_commit

  Scenario: Create non-isolated session without worktree
    Given a git repository at "/project"
    When I create a session with id "def456" and isolated=false
    Then no worktree should be created for session "def456"
    And the session worktree_path should be None

  Scenario: Default session creation is non-isolated
    Given a git repository at "/project"
    When I create a session with id "ghi789" without specifying isolation
    Then no worktree should be created for session "ghi789"
    And the session should behave as isolated=false

  Scenario: effective_cwd returns worktree path for isolated session
    Given a git repository at "/project"
    And an isolated session "abc123" with worktree at ".fspec/worktrees/abc123/"
    When I call effective_cwd on the session
    Then the result should be "/project/.fspec/worktrees/abc123"

  Scenario: effective_cwd returns project root for non-isolated session
    Given a git repository at "/project"
    And a non-isolated session "def456"
    When I call effective_cwd on the session
    Then the result should be "/project"

  Scenario: Create isolated session fails if worktree already exists
    Given a git repository at "/project"
    And a worktree already exists for session "abc123"
    When I try to create a session with id "abc123" and isolated=true
    Then the operation should fail with WorktreeExists error
    And the error should reference session "abc123"

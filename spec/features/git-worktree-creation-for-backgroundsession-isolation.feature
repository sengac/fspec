@session-management
@codelet
@GIT-014
Feature: Git worktree creation for BackgroundSession isolation
  """
  Reuse existing data directory patterns (codelet/common/src/data_dir.rs). Follow DRY/SOLID/composable principles. Worktree operations in own file (codelet/git/src/worktree.rs). Auto-create .fspec/worktrees/ if it doesn't exist.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Worktree metadata must be tracked in agent session data
  #   2. Orphaned worktrees must be cleaned up when sessions end
  #   3. Worktrees ARE needed for true parallel agent work. Ghost commits are for snapshots/undo within a single worktree. Worktrees provide filesystem isolation so multiple sessions can work simultaneously without file conflicts. Codex does use worktree detection code, they just don't create them automatically.
  #   4. MUST use pure gitoxide (gix) Rust implementation - NO git CLI commands allowed anywhere
  #   5. Worktrees use detached HEAD mode (no branch pollution, cleanup handled by GIT-018 Session Manager)
  #   6. Worktrees are created from a specified commit ref (defaults to HEAD). Capturing dirty state before creation is caller's responsibility (via GIT-017 ghost commits).
  #   7. Worktrees are created in .fspec/worktrees/<session-id>/ directory. Work unit association tracked in session metadata, not directory name.
  #   8. Session metadata stays in ~/.fspec/sessions/<session-id>.json (existing system). Worktree path added as field in SessionManifest. Session Manager (GIT-018) uses this to detect orphaned worktrees.
  #
  # EXAMPLES:
  #   1. Spawning an isolated session creates worktree at .fspec/worktrees/<session-uuid>/ based on HEAD
  #   2. Spawning an isolated session with a specific commit ref creates worktree based on that commit
  #   3. Removing a worktree deletes the .fspec/worktrees/<session-id>/ directory and updates git metadata
  #
  # QUESTIONS (ANSWERED):
  #   Q: OpenAI Codex doesn't use worktrees at all - they use Ghost Commits for isolation. Should we reconsider the worktree approach, or is it still needed for true parallel agent work?
  #   A: Worktrees ARE needed for true parallel agent work. Ghost commits are for snapshots/undo within a single worktree. Worktrees provide filesystem isolation so multiple sessions can work simultaneously without file conflicts. Codex does use worktree detection code, they just don't create them automatically.
  #
  # ========================================
  Background: User Story
    As a parent agent
    I want to create isolated git worktrees for spawned child agents
    So that they can work concurrently without conflicting with each other's changes

  # ========================================
  # SCENARIOS
  # ========================================
  @happy-path
  Scenario: Create worktree at HEAD for new session
    Given I have a git repository with commits
    And I have a session ID "abc-123-def"
    When I create a worktree for the session
    Then a worktree should exist at ".fspec/worktrees/abc-123-def/"
    And the worktree should be in detached HEAD mode
    And the worktree HEAD should match the main repository HEAD
    And the session manifest should have the worktree_path field set
    And the session manifest should have the base_commit field set to HEAD
    And the session manifest should have the worktree_created_at timestamp

  @happy-path
  Scenario: Create worktree at specific commit ref
    Given I have a git repository with commits
    And I have a session ID "xyz-456-uvw"
    And I have a commit ref "abc1234"
    When I create a worktree for the session at commit "abc1234"
    Then a worktree should exist at ".fspec/worktrees/xyz-456-uvw/"
    And the worktree should be in detached HEAD mode
    And the worktree HEAD should point to commit "abc1234"

  @happy-path
  Scenario: Auto-create worktrees directory if it doesn't exist
    Given I have a git repository with commits
    And the ".fspec/worktrees/" directory does not exist
    And I have a session ID "first-session"
    When I create a worktree for the session
    Then the ".fspec/worktrees/" directory should be created
    And a worktree should exist at ".fspec/worktrees/first-session/"

  @happy-path
  Scenario: Remove worktree and clean up git metadata
    Given I have a git repository with commits
    And a worktree exists at ".fspec/worktrees/session-to-remove/"
    When I remove the worktree for session "session-to-remove"
    Then the ".fspec/worktrees/session-to-remove/" directory should not exist
    And the git worktree metadata should be cleaned up

  @happy-path
  Scenario: List all session worktrees
    Given I have a git repository with commits
    And a worktree exists at ".fspec/worktrees/session-1/"
    And a worktree exists at ".fspec/worktrees/session-2/"
    When I list all worktrees
    Then I should see 2 worktrees
    And the list should include "session-1"
    And the list should include "session-2"

  @error-handling
  Scenario: Fail gracefully when creating worktree in non-git directory
    Given I have a directory that is not a git repository
    And I have a session ID "orphan-session"
    When I attempt to create a worktree for the session
    Then I should receive an error indicating no git repository found

  @error-handling
  Scenario: Fail gracefully when worktree already exists for session
    Given I have a git repository with commits
    And a worktree already exists at ".fspec/worktrees/existing-session/"
    When I attempt to create a worktree for session "existing-session"
    Then I should receive an error indicating worktree already exists

  @integration
  Scenario: Worktree provides true filesystem isolation
    Given I have a git repository with a file "src/main.rs"
    And a worktree exists at ".fspec/worktrees/isolated-session/"
    When I modify "src/main.rs" in the worktree
    Then the main repository "src/main.rs" should be unchanged
    And the worktree "src/main.rs" should contain my changes

  @happy-path
  Scenario: Session without worktree uses main repository directly
    Given I have a git repository with commits
    And I have a session ID "non-isolated-session"
    When I create a session without worktree isolation
    Then no worktree should be created for the session
    And the session manifest worktree_path field should be null
    And the session should use the main repository working directory

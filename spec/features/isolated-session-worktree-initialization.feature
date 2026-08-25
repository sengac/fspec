@GIT-035
Feature: Isolated session worktree initialization
  """
  Fix worktree.rs/isolated_session.rs to run git reset --mixed HEAD or equivalent gix operation after worktree creation to initialize index
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Worktree creation MUST initialize git index from HEAD so git ls-files returns all tracked files
  #   2. After worktree creation, git status MUST show 'nothing to commit, working tree clean'
  #   3. get_session_diff() MUST detect corrupted index state and report error or repair it
  #   4. Session Management Panel MUST show accurate file change counts matching actual worktree diff
  #
  # EXAMPLES:
  #   1. Create isolated session, run git ls-files in worktree, shows all tracked files (not 0)
  #   2. Create isolated session, run git status in worktree, shows 'nothing to commit, working tree clean'
  #   3. Open Session Management Panel, modify a file in worktree, panel shows '1 files changed'
  #
  # ========================================
  Background: User Story
    As a developer using isolated sessions
    I want to have worktrees created with proper git index
    So that I can safely experiment in isolated worktrees

  # ========================================
  # Part A: Worktree Index Initialization
  # ========================================
  @rust
  @gitoxide
  Scenario: Worktree has all tracked files in git index after creation
    Given I have a git repository with tracked files
    When I create an isolated session
    Then the worktree should exist at ".fspec/worktrees/<session-id>/"
    And "git ls-files" in the worktree should return all tracked files
    And the file count should match the main repository

  @rust
  @gitoxide
  Scenario: Worktree has clean git status after creation
    Given I have a git repository with tracked files
    When I create an isolated session
    Then "git status" in the worktree should show "nothing to commit, working tree clean"
    And there should be no staged changes

  @rust
  @gitoxide
  Scenario: Session Management Panel shows accurate file change count
  # ========================================
  # Part B: Session Diff Accuracy
  # ========================================
    Given I have an isolated session with a worktree
    When I modify a file in the worktree
    And I open the Session Management Panel
    Then the session should show "1 files changed"
    And the modified file should appear in the changes list

  @rust
  @gitoxide
  Scenario: get_session_diff detects and reports corrupted index
    Given I have an isolated session with an empty git index
    When get_session_diff is called for that session
    Then it should detect the corrupted index state
    And it should either report an error or repair the index

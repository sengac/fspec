@GIT-028
Feature: Add createIsolatedSession NAPI binding
  """
  Add session_manager_create_isolated() in rust/napi/src/session_manager.rs, uses IsolatedSessionInfo::new_isolated() and create_session_manifest() from codelet-git
  TypeScript tests must be in src/tui/__tests__/ or similar, calling the actual NAPI binding - NOT Rust source-code-grep tests
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. NAPI binding sessionManagerCreateIsolated(sessionId, model, project, name) must be exposed to TypeScript
  #   2. When called, creates worktree at .fspec/worktrees/<session-id>/ using create_worktree() from codelet-git
  #   3. Creates session manifest via create_session_manifest() for orphan detection
  #   4. Creates BackgroundSession with worktree_path and base_commit populated (not None)
  #   5. Returns session info including worktree path for caller to use
  #   6. If worktree already exists for session ID, returns WorktreeExists error
  #
  # EXAMPLES:
  #   1. Call sessionManagerCreateIsolated('abc-123', 'anthropic/claude', '/project', 'My Session'), worktree created at /project/.fspec/worktrees/abc-123/, session returned with worktree_path set
  #   2. File written via isolated session appears in worktree directory, not in main project
  #   3. Call sessionManagerCreateIsolated with same session ID twice, second call fails with WorktreeExists error
  #   4. After creating isolated session, session manifest exists at ~/.fspec/git-sessions/<session-id>.json
  #   5. After creating isolated session, listSessions() shows session with status 'active'
  #
  # ========================================
  Background: User Story
    As a developer
    I want to create isolated sessions via TypeScript
    So that AI agents can work in git worktrees without affecting the main project

  Scenario: Create isolated session with worktree
    Given a git repository at "/project"
    And no worktree exists for session "abc-123"
    When I call sessionManagerCreateIsolated with session ID "abc-123", model "anthropic/claude", project "/project", and name "My Session"
    Then a worktree should be created at "/project/.fspec/worktrees/abc-123/"
    And the returned session info should include worktree_path
    And the returned session info should include base_commit

  Scenario: Isolated session files appear in worktree not main project
    Given a git repository at "/project"
    And an isolated session "abc-123" has been created
    When a file "test.txt" is written via the isolated session
    Then the file should exist at "/project/.fspec/worktrees/abc-123/test.txt"
    And the file should NOT exist at "/project/test.txt"

  Scenario: Creating duplicate isolated session fails with WorktreeExists error
    Given a git repository at "/project"
    And an isolated session "abc-123" already exists
    When I call sessionManagerCreateIsolated with session ID "abc-123"
    Then the operation should fail with WorktreeExists error
    And the error message should reference session "abc-123"

  Scenario: Session manifest created for orphan detection
    Given a git repository at "/project"
    When I call sessionManagerCreateIsolated with session ID "abc-123"
    Then a session manifest should exist at "~/.fspec/git-sessions/abc-123.json"
    And the manifest should contain the project root path
    And the manifest should contain the worktree path

  Scenario: Isolated session appears in listSessions with active status
    Given a git repository at "/project"
    When I call sessionManagerCreateIsolated with session ID "abc-123"
    And I call listSessions with "abc-123" in the active sessions set
    Then the session should appear in the results
    And the session status should be "active"

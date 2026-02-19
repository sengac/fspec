@session-management
@codelet
@git
@GIT-020
Feature: Isolated Session File Operations
  """
  Tool wrappers need access to session's effective_cwd via callback mechanism
  FileToolFacadeWrapper, BashToolFacadeWrapper already have session_id from TOOL-012 pattern
  Add get_effective_cwd_callback similar to existing get_work_unit_stage_callback pattern
  Direct file tools (Read, Write, Edit) require path rewriting at facade wrapper level
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. File operations (Read, Write, Edit, Bash) use session's effective_cwd for path resolution
  #   2. Relative paths are resolved against effective_cwd, absolute paths within worktree are respected
  #   3. Bash commands execute with cwd set to effective_cwd
  #   4. Multiple isolated sessions can run in parallel without file conflicts
  #   5. Non-isolated sessions continue to use project root as effective_cwd (backward compatible)
  #
  # EXAMPLES:
  #   1. Isolated session writes file, file exists in worktree only
  #   2. Isolated session reads file, gets content from worktree
  #   3. Bash 'pwd' in isolated session returns worktree path
  #   4. Two parallel sessions write to same relative path without conflict
  #   5. Edit tool modifies file in worktree, main project unchanged
  #   6. Non-isolated session file operations affect main project (backward compatible)
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want to perform file operations in my session's effective_cwd
    So that my file writes and reads are isolated to my worktree and don't affect the main project or other sessions

  # ========================================
  # SCENARIOS
  # ========================================
  @write
  @isolation
  Scenario: File written by isolated session appears in worktree only
    Given I am an AI agent running in an isolated session
    And my session has worktree_path "/project/.fspec/worktrees/abc123"
    When I write content "test content" to file "src/new-file.ts"
    Then the file should exist at "/project/.fspec/worktrees/abc123/src/new-file.ts"
    And the file should NOT exist at "/project/src/new-file.ts"

  @read
  @isolation
  Scenario: File read by isolated session comes from worktree
    Given I am an AI agent running in an isolated session
    And my session has worktree_path "/project/.fspec/worktrees/abc123"
    And a file exists at "/project/.fspec/worktrees/abc123/src/config.ts" with content "worktree content"
    And a file exists at "/project/src/config.ts" with content "main project content"
    When I read file "src/config.ts"
    Then the content should be "worktree content"

  @bash
  @isolation
  Scenario: Bash pwd in isolated session returns worktree path
    Given I am an AI agent running in an isolated session
    And my session has worktree_path "/project/.fspec/worktrees/abc123"
    When I execute bash command "pwd"
    Then the output should contain "/project/.fspec/worktrees/abc123"

  @parallel
  @isolation
  Scenario: Two parallel sessions write to same relative path without conflict
    Given two AI agent sessions are running in parallel
    And session A has worktree_path "/project/.fspec/worktrees/session-a"
    And session B has worktree_path "/project/.fspec/worktrees/session-b"
    When session A writes "content A" to file "src/shared.ts"
    And session B writes "content B" to file "src/shared.ts"
    Then "/project/.fspec/worktrees/session-a/src/shared.ts" should contain "content A"
    And "/project/.fspec/worktrees/session-b/src/shared.ts" should contain "content B"
    And the files should be independent

  @edit
  @isolation
  Scenario: Edit tool modifies file in worktree only
    Given I am an AI agent running in an isolated session
    And my session has worktree_path "/project/.fspec/worktrees/abc123"
    And a file exists at "/project/.fspec/worktrees/abc123/src/app.ts" with content "original"
    And a file exists at "/project/src/app.ts" with content "main original"
    When I edit file "src/app.ts" replacing "original" with "modified"
    Then "/project/.fspec/worktrees/abc123/src/app.ts" should contain "modified"
    And "/project/src/app.ts" should still contain "main original"

  @backward-compatible
  Scenario: Non-isolated session file operations affect main project
    Given I am an AI agent running in a non-isolated session
    And my session has NO worktree_path
    When I write content "new content" to file "src/test.ts"
    Then the file should exist at "/project/src/test.ts"
    And there should be NO worktree directory

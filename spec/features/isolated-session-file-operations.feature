@session-management
@codelet
@git
@GIT-020
@e2e
Feature: Isolated Session File Operations - BLOCKING Access to Main Project
  """
  CRITICAL SECURITY REQUIREMENT:
  Isolated sessions MUST be BLOCKED from accessing files outside the worktree.
  This feature tests the BLOCKING behavior end-to-end using real NAPI bindings.

  Tests MUST NOT use mocks, stubs, or test doubles for the isolation mechanism.
  Tests MUST create real isolated sessions via NAPI, invoke real tools, verify blocking.

  The path validation happens in validate_and_resolve_path() in wrapper.rs.
  The get_session_effective_cwd callback in session_manager.rs links session→worktree.
  E2E tests verify this wiring works correctly.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   5. CRITICAL: Isolated sessions MUST be BLOCKED from reading files outside worktree via Read tool
  #   6. CRITICAL: Isolated sessions MUST be BLOCKED from writing files outside worktree via Write tool
  #   7. CRITICAL: Isolated sessions MUST be BLOCKED from editing files outside worktree via Edit tool
  #   8. CRITICAL: Isolated sessions MUST be BLOCKED from listing directories outside worktree via Ls tool
  #   9. CRITICAL: Isolated sessions MUST be BLOCKED from searching files outside worktree via Grep tool
  #   10. CRITICAL: Isolated sessions MUST be BLOCKED from globbing files outside worktree via Glob tool
  #   11. CRITICAL: Isolated sessions MUST be BLOCKED from AST searching outside worktree via AstGrep tool
  #   12. CRITICAL: Isolated sessions MUST be BLOCKED from AST refactoring outside worktree via AstGrepRefactor tool
  #   13. CRITICAL: Isolated sessions Bash commands MUST execute with cwd restricted to worktree
  #   14. Blocking must work for absolute paths pointing to main project
  #   15. Blocking must work for path traversal attempts (../../)
  #   16. Blocking must work for symlink attacks
  #   17. Non-isolated sessions MUST NOT be affected (backward compatible)
  #   18. Tests MUST be E2E: create real isolated session via NAPI, invoke real tools, verify blocking
  #   19. Tests MUST NOT use mocks, stubs, or test doubles for the isolation mechanism
  #
  # EXAMPLES:
  #   6. E2E: Isolated session Read tool on /project/src/main.ts → BLOCKED with error
  #   7. E2E: Isolated session Write tool on /project/src/new.ts → BLOCKED with error
  #   8. E2E: Isolated session Edit tool on /project/src/existing.ts → BLOCKED with error
  #   9. E2E: Isolated session Ls tool on /project/src/ → BLOCKED with error
  #   10. E2E: Isolated session Grep tool with path=/project/src/ → BLOCKED with error
  #   11. E2E: Isolated session Glob tool with path=/project/ → BLOCKED with error
  #   12. E2E: Isolated session AstGrep tool with path=/project/ → BLOCKED with error
  #   13. E2E: Isolated session AstGrepRefactor tool on /project/src/file.ts → BLOCKED with error
  #   14. E2E: Isolated session Read tool on worktree relative path → ALLOWED
  #   15. E2E: Isolated session Read tool on worktree absolute path → ALLOWED
  #   16. E2E: Path traversal ../../src/main.ts → BLOCKED
  #   17. E2E: Symlink attack → BLOCKED
  #   18. E2E: Non-isolated session Read on main project → ALLOWED (backward compatible)
  #   19. E2E: Non-isolated session Write on main project → ALLOWED (backward compatible)
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want to be blocked from accessing files outside my worktree when running in an isolated session
    So that my changes stay isolated and I cannot accidentally read or modify the main project

  # ========================================
  # BLOCKING SCENARIOS - Read Tool
  # ========================================
  @read
  @blocking
  @critical
  Scenario: Isolated session Read tool BLOCKED from reading main project file with absolute path
    Given a git repository at "/project" with file "/project/src/main.ts" containing "main project content"
    And an isolated session is created via sessionManagerCreateIsolated NAPI binding
    And the session has worktree at "/project/.fspec/worktrees/<session-id>"
    When the Read tool is invoked with file_path "/project/src/main.ts"
    Then the tool should return an error containing "outside isolated worktree"
    And the file should NOT be read
    And a block notification should be emitted

  @read
  @blocking
  @path-traversal
  Scenario: Isolated session Read tool BLOCKED from path traversal escape
    Given a git repository at "/project" with file "/project/src/main.ts" containing "main project content"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    And the worktree contains directory "src/"
    When the Read tool is invoked with file_path "../../src/main.ts"
    Then the tool should return an error containing "outside isolated worktree"
    And the file should NOT be read

  @read
  @blocking
  @symlink
  Scenario: Isolated session Read tool BLOCKED from symlink escape
    Given a git repository at "/project" with file "/project/src/secret.ts" containing "secret content"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    And the worktree contains a symlink "escape" pointing to "/project/src/"
    When the Read tool is invoked with file_path "escape/secret.ts"
    Then the tool should return an error containing "outside isolated worktree"
    And the file should NOT be read

  @read
  @allowed
  Scenario: Isolated session Read tool ALLOWED for relative path within worktree
    Given a git repository at "/project"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    And the worktree contains file "src/app.ts" with content "worktree content"
    When the Read tool is invoked with file_path "src/app.ts"
    Then the tool should succeed
    And the content should be "worktree content"

  @read
  @allowed
  Scenario: Isolated session Read tool ALLOWED for absolute path within worktree
    Given a git repository at "/project"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    And the worktree contains file "src/app.ts" with content "worktree content"
    When the Read tool is invoked with file_path "/project/.fspec/worktrees/<session-id>/src/app.ts"
    Then the tool should succeed
    And the content should be "worktree content"

  # ========================================
  # BLOCKING SCENARIOS - Write Tool
  # ========================================
  @write
  @blocking
  @critical
  Scenario: Isolated session Write tool BLOCKED from writing to main project
    Given a git repository at "/project"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the Write tool is invoked with file_path "/project/src/malicious.ts" and content "injected code"
    Then the tool should return an error containing "outside isolated worktree"
    And the file should NOT exist at "/project/src/malicious.ts"
    And a block notification should be emitted

  @write
  @allowed
  Scenario: Isolated session Write tool ALLOWED for relative path within worktree
    Given a git repository at "/project"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the Write tool is invoked with file_path "src/new-file.ts" and content "new content"
    Then the tool should succeed
    And the file should exist at worktree path "src/new-file.ts" with content "new content"
    And the file should NOT exist at "/project/src/new-file.ts"

  # ========================================
  # BLOCKING SCENARIOS - Edit Tool
  # ========================================
  @edit
  @blocking
  @critical
  Scenario: Isolated session Edit tool BLOCKED from editing main project file
    Given a git repository at "/project" with file "/project/src/config.ts" containing "original"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the Edit tool is invoked with file_path "/project/src/config.ts" replacing "original" with "modified"
    Then the tool should return an error containing "outside isolated worktree"
    And the file at "/project/src/config.ts" should still contain "original"
    And a block notification should be emitted

  # ========================================
  # BLOCKING SCENARIOS - Ls Tool
  # ========================================
  @ls
  @blocking
  @critical
  Scenario: Isolated session Ls tool BLOCKED from listing main project directory
    Given a git repository at "/project" with directory "/project/src/" containing files
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the Ls tool is invoked with path "/project/src/"
    Then the tool should return an error containing "outside isolated worktree"

  @ls
  @allowed
  Scenario: Isolated session Ls tool ALLOWED for worktree directory
    Given a git repository at "/project"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    And the worktree contains directory "src/" with files
    When the Ls tool is invoked with path "src/"
    Then the tool should succeed
    And the output should list files in the worktree src/ directory

  # ========================================
  # BLOCKING SCENARIOS - Grep Tool
  # ========================================
  @grep
  @blocking
  @critical
  Scenario: Isolated session Grep tool BLOCKED from searching main project
    Given a git repository at "/project" with files containing searchable content
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the Grep tool is invoked with pattern "TODO" and path "/project/src/"
    Then the tool should return an error containing "outside isolated worktree"

  @grep
  @allowed
  Scenario: Isolated session Grep tool ALLOWED for searching worktree
    Given a git repository at "/project"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    And the worktree contains file "src/app.ts" with content "// FIXME: fix this"
    When the Grep tool is invoked with pattern "FIXME" and path "src/"
    Then the tool should succeed
    And the results should include matches from worktree

  # ========================================
  # BLOCKING SCENARIOS - Glob Tool
  # ========================================
  @glob
  @blocking
  @critical
  Scenario: Isolated session Glob tool BLOCKED from globbing main project
    Given a git repository at "/project" with TypeScript files
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the Glob tool is invoked with pattern "**/*.ts" and path "/project/"
    Then the tool should return an error containing "outside isolated worktree"

  @glob
  @allowed
  Scenario: Isolated session Glob tool ALLOWED for globbing worktree
    Given a git repository at "/project"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    And the worktree contains TypeScript files in "src/"
    When the Glob tool is invoked with pattern "**/*.ts" and path "src/"
    Then the tool should succeed
    And the results should only include worktree files

  # ========================================
  # BLOCKING SCENARIOS - AstGrep Tool
  # ========================================
  @astgrep
  @blocking
  @critical
  Scenario: Isolated session AstGrep tool BLOCKED from searching main project
    Given a git repository at "/project" with TypeScript files
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the AstGrep tool is invoked with pattern "function $NAME()" language "typescript" and path "/project/"
    Then the tool should return an error containing "outside isolated worktree"

  # ========================================
  # BLOCKING SCENARIOS - AstGrepRefactor Tool
  # ========================================
  @astgrep-refactor
  @blocking
  @critical
  Scenario: Isolated session AstGrepRefactor tool BLOCKED from refactoring main project
    Given a git repository at "/project" with file "/project/src/refactor-me.ts"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the AstGrepRefactor tool is invoked with source_file "/project/src/refactor-me.ts"
    Then the tool should return an error containing "outside isolated worktree"
    And the file at "/project/src/refactor-me.ts" should be unchanged

  # ========================================
  # BACKWARD COMPATIBILITY - Non-Isolated Sessions
  # ========================================
  @backward-compatible
  @non-isolated
  Scenario: Non-isolated session Read tool ALLOWED for all paths
    Given a git repository at "/project" with file "/project/src/main.ts" containing "main content"
    And a non-isolated session is created via sessionManagerCreateWithId NAPI binding
    When the Read tool is invoked with file_path "/project/src/main.ts"
    Then the tool should succeed
    And the content should be "main content"

  @backward-compatible
  @non-isolated
  Scenario: Non-isolated session Write tool ALLOWED for all paths
    Given a git repository at "/project"
    And a non-isolated session is created via sessionManagerCreateWithId NAPI binding
    When the Write tool is invoked with file_path "/project/src/new.ts" and content "new content"
    Then the tool should succeed
    And the file should exist at "/project/src/new.ts" with content "new content"

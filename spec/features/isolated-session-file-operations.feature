@security
@done
@session-management
@codelet
@git
@GIT-020
@e2e
Feature: Isolated Session File Operations - BLOCKING Access to Original Project Only
  """
  CRITICAL BUG FIX: Isolation was blocking ALL paths outside worktree (TOO RESTRICTIVE).

  CORRECT BEHAVIOR:
  - ALLOW: Paths within the worktree (e.g., /project/.fspec/worktrees/xyz/src/file.ts)
  - BLOCK: Paths within the ORIGINAL PROJECT that the worktree was created from (e.g., /project/src/file.ts)
  - ALLOW: All other paths on the filesystem (e.g., /tmp, /Users/rquast/Desktop/, /etc)

  The session stores BOTH:
  - `project: String` - the original project path (BLOCKED)
  - `worktree_path: Option<PathBuf>` - the isolated worktree path (ALLOWED)

  Path validation logic:
  1. If path resolves to within worktree → ALLOW
  2. If path resolves to within original project → BLOCK
  3. Otherwise (e.g., /tmp, /etc) → ALLOW

  Tests MUST NOT use mocks, stubs, or test doubles for the isolation mechanism.
  Tests MUST create real isolated sessions via NAPI, invoke real tools, verify behavior.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Isolated sessions BLOCK paths within the original project directory
  #   2. Isolated sessions ALLOW paths within the worktree
  #   3. Isolated sessions ALLOW all other paths (/tmp, /etc, home directory)
  #   4. Non-isolated sessions have no path restrictions (backward compatible)
  #   5. Path traversal (../../) resolving to original project is BLOCKED
  #   6. Symlinks pointing to original project are BLOCKED (symlink escape attack)
  #   7. All file tools (Read, Write, Edit, Ls, Grep, Glob, AstGrep, AstGrepRefactor) must enforce these rules
  #   8. Bash tool cwd is restricted to worktree for isolated sessions
  #
  # EXAMPLES:
  #   1. Read /tmp/file.txt from isolated session → ALLOWED
  #   2. Read /project/src/main.ts (original project) from isolated session → BLOCKED
  #   3. Read worktree/src/main.ts (within worktree) from isolated session → ALLOWED
  #   4. Non-isolated session reads ANY path → ALLOWED
  #   5. Read ../../src/main.ts from worktree (resolves to original project) → BLOCKED
  #   6. Read via symlink in worktree pointing to original project → BLOCKED
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want file operations in my isolated session to be blocked ONLY from the original project directory
    So that I cannot accidentally modify the main project, but can still access system paths like /tmp

  # ========================================
  # BLOCKING SCENARIOS - Read Tool (Original Project)
  # ========================================
  @read
  @blocking
  @critical
  Scenario: Isolated session Read tool BLOCKED from reading original project file
    Given a git repository at "/project" with file "/project/src/main.ts" containing "main project content"
    And an isolated session is created via sessionManagerCreateIsolated NAPI binding
    And the session has worktree at "/project/.fspec/worktrees/<session-id>"
    When the Read tool is invoked with file_path "/project/src/main.ts"
    Then the tool should return an error containing "blocked from original project"
    And the file should NOT be read
    And a block notification should be emitted

  @read
  @blocking
  @path-traversal
  Scenario: Isolated session Read tool BLOCKED from path traversal to original project
    Given a git repository at "/project" with file "/project/src/main.ts" containing "main project content"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    And the worktree contains directory "src/"
    When the Read tool is invoked with file_path "../../src/main.ts"
    Then the tool should return an error containing "blocked from original project"
    And the file should NOT be read

  @read
  @blocking
  @symlink
  Scenario: Isolated session Read tool BLOCKED from symlink escape to original project
    Given a git repository at "/project" with file "/project/src/secret.ts" containing "secret content"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    And the worktree contains a symlink "escape" pointing to "/project/src/"
    When the Read tool is invoked with file_path "escape/secret.ts"
    Then the tool should return an error containing "blocked from original project"
    And the file should NOT be read

  # ========================================
  # ALLOWED SCENARIOS - Read Tool (Within Worktree)
  # ========================================
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
  # ALLOWED SCENARIOS - Read Tool (Outside Project - /tmp, /etc)
  # ========================================
  @read
  @allowed
  @filesystem-access
  Scenario: Isolated session Read tool ALLOWED for /tmp (not in original project)
    Given an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    And a file exists at "/tmp/test-file.txt" with content "temp content"
    When the Read tool is invoked with file_path "/tmp/test-file.txt"
    Then the tool should succeed
    And the content should be "temp content"

  @read
  @allowed
  @filesystem-access
  Scenario: Isolated session Ls tool ALLOWED for /tmp directory
    Given an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the Ls tool is invoked with path "/tmp"
    Then the tool should succeed
    And the output should list directory contents

  @grep
  @allowed
  @filesystem-access
  Scenario: Isolated session Grep tool ALLOWED for searching /tmp
    Given an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    And a file exists at "/tmp/searchable.txt" with content "findme pattern"
    When the Grep tool is invoked with pattern "findme" and path "/tmp"
    Then the tool should succeed
    And the results should include matches from /tmp

  # ========================================
  # BLOCKING SCENARIOS - Write Tool (Original Project)
  # ========================================
  @write
  @blocking
  @critical
  Scenario: Isolated session Write tool BLOCKED from writing to original project
    Given a git repository at "/project"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the Write tool is invoked with file_path "/project/src/malicious.ts" and content "injected code"
    Then the tool should return an error containing "blocked from original project"
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

  @write
  @allowed
  @filesystem-access
  Scenario: Isolated session Write tool ALLOWED for /tmp (not blocked by isolation)
    Given an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the Write tool is invoked with file_path "/tmp/test-write.txt" and content "written content"
    Then the tool should succeed
    And the file should exist at "/tmp/test-write.txt" with content "written content"

  # ========================================
  # BLOCKING SCENARIOS - Edit Tool (Original Project)
  # ========================================
  @edit
  @blocking
  @critical
  Scenario: Isolated session Edit tool BLOCKED from editing original project file
    Given a git repository at "/project" with file "/project/src/config.ts" containing "original"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the Edit tool is invoked with file_path "/project/src/config.ts" replacing "original" with "modified"
    Then the tool should return an error containing "blocked from original project"
    And the file at "/project/src/config.ts" should still contain "original"
    And a block notification should be emitted

  # ========================================
  # BLOCKING SCENARIOS - Ls Tool (Original Project)
  # ========================================
  @ls
  @blocking
  @critical
  Scenario: Isolated session Ls tool BLOCKED from listing original project directory
    Given a git repository at "/project" with directory "/project/src/" containing files
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the Ls tool is invoked with path "/project/src/"
    Then the tool should return an error containing "blocked from original project"

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
  # BLOCKING SCENARIOS - Grep Tool (Original Project)
  # ========================================
  @grep
  @blocking
  @critical
  Scenario: Isolated session Grep tool BLOCKED from searching original project
    Given a git repository at "/project" with files containing searchable content
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the Grep tool is invoked with pattern "TODO" and path "/project/src/"
    Then the tool should return an error containing "blocked from original project"

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
  # BLOCKING SCENARIOS - Glob Tool (Original Project)
  # ========================================
  @glob
  @blocking
  @critical
  Scenario: Isolated session Glob tool BLOCKED from globbing original project
    Given a git repository at "/project" with TypeScript files
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the Glob tool is invoked with pattern "**/*.ts" and path "/project/"
    Then the tool should return an error containing "blocked from original project"

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
  # BLOCKING SCENARIOS - AstGrep Tool (Original Project)
  # ========================================
  @astgrep
  @blocking
  @critical
  Scenario: Isolated session AstGrep tool BLOCKED from searching original project
    Given a git repository at "/project" with TypeScript files
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the AstGrep tool is invoked with pattern "function $NAME()" language "typescript" and path "/project/"
    Then the tool should return an error containing "blocked from original project"

  # ========================================
  # BLOCKING SCENARIOS - AstGrepRefactor Tool (Original Project)
  # ========================================
  @astgrep-refactor
  @blocking
  @critical
  Scenario: Isolated session AstGrepRefactor tool BLOCKED from refactoring original project
    Given a git repository at "/project" with file "/project/src/refactor-me.ts"
    And an isolated session with worktree at "/project/.fspec/worktrees/<session-id>"
    When the AstGrepRefactor tool is invoked with source_file "/project/src/refactor-me.ts"
    Then the tool should return an error containing "blocked from original project"
    And the file at "/project/src/refactor-me.ts" should be unchanged

  # ========================================
  # BACKWARD COMPATIBILITY - Non-Isolated Sessions
  # ========================================
  @backward-compatible
  @non-isolated
  Scenario: Non-isolated session Read tool ALLOWED for all paths including original project
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

  @backward-compatible
  @non-isolated
  Scenario: Non-isolated session can access /tmp, /etc, anywhere
    Given a non-isolated session is created via sessionManagerCreateWithId NAPI binding
    And a file exists at "/tmp/anywhere.txt" with content "accessible"
    When the Read tool is invoked with file_path "/tmp/anywhere.txt"
    Then the tool should succeed
    And the content should be "accessible"

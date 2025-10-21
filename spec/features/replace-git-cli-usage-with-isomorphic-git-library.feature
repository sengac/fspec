@done
@migration
@isomorphic-git
@infrastructure
@git
@phase1
@GIT-001
Feature: Replace git CLI usage with isomorphic-git library
  """
  Uses isomorphic-git pure JavaScript implementation for all git operations. Creates modular abstraction layer in src/git/ with separate files for status, add, commit, and log operations. Replaces all execa-based git CLI calls while keeping execa for lifecycle hooks. Provides semantic wrapper types (e.g., FileStatus) to hide isomorphic-git implementation details. Supports configurable strict mode for error handling. Uses memfs for unit testing with Vitest. See attachments for comprehensive implementation guide and testing patterns.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Must create comprehensive git abstraction layer in src/git/ with common operations (status, add, commit, log) following best practices patterns
  #   2. Git abstraction layer must be architected as core shared infrastructure for all git operations across fspec
  #   3. Keep execa dependency (used for lifecycle hooks execution) but replace all git CLI calls with isomorphic-git
  #   4. Git abstraction layer must support configurable strict mode - callers can choose whether errors are thrown or silently return empty results
  #   5. No barrel exports (index.ts) - consumers must import directly from specific modules like src/git/status.ts
  #   6. Use modular structure with separate files per operation area: status.ts, add.ts, commit.ts, log.ts in src/git/ directory
  #   7. Create semantic wrapper types that hide isomorphic-git implementation details - consumers should not depend on library-specific types like StatusRow
  #   8. Internal modules can use isomorphic-git types internally but must transform to semantic types (e.g., FileStatus) at module boundaries
  #   9. Use isomorphic-git testing patterns from official documentation - research and document mock filesystem approach for unit tests
  #   10. No performance benchmarking required - isomorphic-git performance is acceptable for fspec use case
  #   11. Complete replacement of git-context.ts implementation - no feature flags or parallel implementations
  #   12. Must update existing feature file, tests, and coverage mappings to align with new isomorphic-git implementation
  #
  # EXAMPLES:
  #   1. Virtual hook with git-context flag needs to get list of staged and unstaged files to pass to hook script (e.g., eslint only on changed files)
  #   2. Checkpoint system (GIT-002 dependency) needs to detect all modified, staged, and untracked files before creating intelligent stash snapshot
  #   3. Empty repository (git init, no commits yet) - should handle gracefully, treat all files as untracked without crashing
  #   4. Repository with .gitignore - operations should respect .gitignore by default, exclude ignored files (node_modules, dist, etc.) from status results
  #   5. Clean repository (no changes) - returns empty arrays efficiently for staged, unstaged, and untracked files
  #
  # ========================================
  Background: User Story
    As a developer maintaining fspec
    I want to replace git CLI calls with isomorphic-git library
    So that fspec can be bundled as a single executable without external dependencies

  Scenario: Get staged and unstaged files for virtual hooks
    Given a git repository with src/git/status.ts module
    And the repository has staged files and unstaged files
    When git-context.ts calls getStagedFiles() and getUnstagedFiles()
    Then it receives arrays of filenames from isomorphic-git status operations
    And the files are passed to virtual hook scripts (e.g., eslint)
    And no git CLI commands are executed via execa

  Scenario: Detect all file changes for checkpoint system
    Given a git repository with modified, staged, and untracked files
    When checkpoint system (GIT-002) calls git status operations
    Then getStagedFiles() returns list of staged files
    And getUnstagedFiles() returns list of modified but unstaged files
    And getUntrackedFiles() returns list of untracked files
    And all operations use isomorphic-git instead of git CLI

  Scenario: Handle empty repository without crashing
    Given a git repository created with git init
    And the repository has no commits (no HEAD)
    And the repository contains untracked files
    When git status operations are called
    Then operations complete successfully without errors
    And all files are correctly identified as untracked
    And no crashes occur from missing HEAD reference

  Scenario: Respect .gitignore when listing files
    Given a git repository with .gitignore file
    And the .gitignore excludes node_modules/ and *.log files
    And the repository contains both tracked and ignored files
    When git status operations are called
    Then results exclude files matching .gitignore patterns
    And node_modules/ files are not listed
    And *.log files are not listed
    And only non-ignored files appear in status results

  Scenario: Return empty arrays for clean repository
    Given a git repository with committed files
    And the working directory matches HEAD (no changes)
    When getStagedFiles(), getUnstagedFiles(), and getUntrackedFiles() are called
    Then all functions return empty arrays
    And operations complete efficiently without errors
    And no unnecessary processing occurs for clean state

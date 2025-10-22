@validation
@done
@testing
@cli
@isomorphic-git
@git
@bug-fix
@refactor
@infrastructure
@GIT-003
Feature: Fix GIT-001 critical bugs and logic errors
  """
  Fixes critical bugs in src/git/status.ts identified through ULTRATHINK analysis. Addresses memfs integration issues, status matrix logic errors, type safety violations, and missing test coverage. See spec/attachments/GIT-003/git-001-critical-analysis.md for complete analysis. Must maintain backward compatibility with existing git-context.ts consumers.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. BLOCKER (S1): Fix failing test - getUnstagedFiles() returns empty array for modified files in memfs environment
  #   2. CRITICAL (S2): Fix logic bug - getUnstagedFiles() misses files that are staged then modified again (partial staging scenario)
  #   3. CRITICAL (S2): Fix FileStatus.modified semantic ambiguity - rename to hasUnstagedModifications or add separate hasUnstagedChanges field
  #   4. HIGH (S3): Replace fs parameter type 'any' with proper IFs interface type for type safety
  #   5. MEDIUM (S4): Add comprehensive test coverage for 14 missing scenarios including deleted files, nested directories, partial staging, and edge cases
  #
  # EXAMPLES:
  #   1. Failing test: File modified after commit in memfs not detected by getUnstagedFiles() - returns [] instead of ['modified.txt']
  #   2. Partial staging miss: echo v2 > file.txt && git add file.txt && echo v3 >> file.txt results in file appearing in staged but NOT unstaged (should be in both)
  #   3. Type safety violation: Passing { someMethod: () => {} } as fs option causes runtime error instead of compile-time error
  #
  # ========================================
  Background: User Story
    As a developer maintaining fspec
    I want to fix critical bugs and logic errors in GIT-001 isomorphic-git integration
    So that virtual hooks and checkpoint system work correctly with all git scenarios

  Scenario: Fix getUnstagedFiles() memfs detection bug
    Given a git repository with a committed file in memfs
    And the file is modified after commit using fs.writeFileSync()
    When getUnstagedFiles() is called
    Then the modified file should be detected and included in results
    And the function should return ['modified.txt'] instead of []

  Scenario: Fix partial staging detection (staged then modified)
    Given a git repository with a committed file
    And the file is modified and staged (git add)
    And the file is modified again after staging
    When getStagedFiles() and getUnstagedFiles() are called
    Then the file should appear in BOTH staged and unstaged lists
    And staged should contain the file (has staged changes)
    And unstaged should contain the file (has unstaged changes)

  Scenario: Replace any type with IFs interface for type safety
    Given the fs parameter in GitStatusOptions uses 'any' type
    When the type is changed to proper IFs interface from memfs
    Then TypeScript should catch invalid fs implementations at compile time
    And passing an object missing required methods should cause type error
    And IntelliSense should provide autocomplete for fs methods

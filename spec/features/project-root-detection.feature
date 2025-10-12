@done
@safety
@file-ops
@phase1
Feature: Prevent spec directory creation outside project root
  """
  Architecture notes:
  - Centralized utility function (findOrCreateSpecDirectory) for all spec/ directory operations
  - Searches upward from cwd for project boundary markers: .git, package.json, .gitignore, Cargo.toml, pyproject.toml
  - Stops at first boundary marker found (closest to cwd)
  - Maximum 10 directories upward search limit to prevent excessive filesystem traversal
  - Returns spec path (not project root) and automatically creates spec/ directory if needed
  - Graceful fallback to cwd on permission errors or filesystem issues
  - Must maintain backward compatibility - all existing tests must continue passing

  Critical implementation requirements:
  - MUST NOT break existing functionality
  - MUST support both old and new code paths during transition
  - MUST handle monorepo scenarios (multiple package.json files)
  - MUST handle edge cases: deep nesting, no markers, permission errors
  - All existing ensure-files.ts callers must work without modification initially

  Search priority (first match wins):
  1. If spec/ exists within project boundary → use it
  2. If no spec/ found → create at boundary marker location
  3. If no boundary marker found → create at cwd (fallback)
  """

  Background: User Story
    As a developer using fspec in various project structures
    I want spec/ directories to be created at the correct project root
    So that specifications stay organized within project boundaries and don't pollute parent directories

  @SAFE-001
  Scenario: Create spec at project root when .git marker exists
    Given I am in directory "/project/src/commands/"
    And a ".git" directory exists at "/project/.git"
    And no "spec" directory exists anywhere
    When I call findOrCreateSpecDirectory()
    Then the function should return "/project/spec/"
    And the directory "/project/spec/" should be created
    And all existing tests should continue passing

  @SAFE-001
  Scenario: Use existing spec directory within project boundary
    Given I am in directory "/project/src/commands/"
    And a ".git" directory exists at "/project/.git"
    And a "spec" directory exists at "/project/spec/"
    When I call findOrCreateSpecDirectory()
    Then the function should return "/project/spec/"
    And no new directories should be created
    And all existing tests should continue passing

  @SAFE-001
  Scenario: Fallback to cwd when no project boundary markers found
    Given I am in directory "/tmp/random/"
    And no project boundary markers exist in parent directories
    When I call findOrCreateSpecDirectory()
    Then the function should return "/tmp/random/spec/"
    And the directory "/tmp/random/spec/" should be created
    And all existing tests should continue passing

  @SAFE-001
  Scenario: Handle monorepo with nested package.json correctly
    Given I am in directory "/monorepo/packages/app/src/"
    And a ".git" directory exists at "/monorepo/.git"
    And a "package.json" file exists at "/monorepo/packages/app/package.json"
    And no "spec" directory exists anywhere
    When I call findOrCreateSpecDirectory()
    Then the function should return "/monorepo/packages/app/spec/"
    And the directory "/monorepo/packages/app/spec/" should be created
    And the function should stop at the first boundary marker (closest to cwd)
    And all existing tests should continue passing

  @SAFE-001
  Scenario: Stop search after maximum directory traversal limit
    Given I am in a very deeply nested directory (more than 10 levels deep)
    And no project boundary markers exist within 10 parent directories
    When I call findOrCreateSpecDirectory()
    Then the function should return a spec path at the current working directory
    And the search should stop after checking 10 parent directories
    And all existing tests should continue passing

  @SAFE-001
  Scenario: Gracefully handle permission errors when searching parent directories
    Given I am in directory "/project/src/"
    And reading parent directories results in permission errors
    When I call findOrCreateSpecDirectory()
    Then the function should gracefully fall back to creating spec at cwd
    And the function should return "/project/src/spec/"
    And the directory "/project/src/spec/" should be created
    And all existing tests should continue passing

@REFAC-009
Feature: Remove unused dependencies from Cargo.toml files

  """
  Dead dependency removal - verified via grep for 'use <crate>::' patterns. Some deps are transitive (grep-matcher for grep-searcher, ast-grep-core for ast-grep-language) and should be kept.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Remove only dependencies that are confirmed unused (not imported anywhere in source code)
  #   2. Keep transitive dependencies that are required by other crates (grep-matcher, ast-grep-core)
  #   3. All existing tests must continue to pass after removal
  #   4. Project must compile successfully with cargo build
  #
  # EXAMPLES:
  #   1. Remove dotenv from workspace - dotenvy is used instead
  #   2. Remove directories from workspace - dirs crate is used instead
  #   3. Remove owo-colors and supports-color - never imported in any source file
  #   4. Remove diffy, walkdir, once_cell, indexmap, sysinfo, toml_edit, which - never imported
  #   5. Remove unused dev deps: pretty_assertions, mockall, insta, proptest - not used in tests
  #   6. cargo test and cargo build pass after all removals
  #
  # ========================================

  Background: User Story
    As a developer maintaining codelet
    I want to remove unused dependencies from Cargo.toml files
    So that I reduce compile time, binary size, and dependency complexity

  Scenario: Remove redundant dotenv dependency
    Given the workspace Cargo.toml contains dotenv and dotenvy
    When I remove the dotenv dependency
    Then the project compiles successfully
    And dotenvy is used in cli/src/main.rs


  Scenario: Remove redundant directories dependency
    Given the workspace Cargo.toml contains directories and dirs
    When I remove the directories dependency
    Then the project compiles successfully
    And dirs is used in cli and common crates


  Scenario: Remove unused terminal color dependencies
    Given owo-colors and supports-color are in workspace Cargo.toml
    When I remove owo-colors and supports-color
    Then the project compiles successfully
    And neither is imported in any source file


  Scenario: Remove unused utility dependencies
    Given diffy, walkdir, once_cell, indexmap, sysinfo, toml_edit, which are declared
    When I remove all unused utility dependencies
    Then the project compiles successfully
    And none are imported in any source file


  Scenario: Remove unused dev dependencies
    Given pretty_assertions, mockall, insta, proptest are in dev-dependencies
    When I remove all unused dev dependencies
    Then all tests still pass
    And none are used in any test file


  Scenario: Project builds and tests pass after cleanup
    Given all unused dependencies have been removed
    When I run cargo build and cargo test
    Then the build succeeds with no errors
    And all tests pass


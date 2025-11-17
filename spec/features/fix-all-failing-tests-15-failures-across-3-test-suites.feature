@validator
@test-maintenance
@testing-framework
@bug
@testing
@critical
Feature: Fix all failing tests (15 failures across 3 test suites)
  """
  Test failure breakdown: 8 AST tests (outdated), 5 TUI tests (needs investigation), 2 research integration tests (needs investigation). AST tests written for stub implementation before RES-016 connected real tree-sitter parser. Comprehensive analysis document attached at spec/attachments/TEST-008/test-failures-analysis.md with detailed recommendations.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ALL tests must be passing (0 failures) before this work unit can be marked as done. Run 'npm test' and verify all test suites pass with 100% success rate.
  #   2. AST Research Tool tests (8 failures in src/__tests__/ast-research-tool.test.ts) are OUTDATED - written for old stub implementation. These tests must be DELETED and REWRITTEN for actual tree-sitter parser behavior.
  #   3. TUI formatting tests (5 failures in src/tui/__tests__/work-unit-details-formatting.test.tsx) need investigation - determine if implementation regressed or test expectations are outdated, then fix whichever is wrong.
  #   4. Research integration tests (2 failures in src/commands/__tests__/integrate-research-guidance.test.ts) need investigation - verify if bootstrap/CLAUDE.md features still exist, update tests accordingly or remove if deprecated.
  #
  # EXAMPLES:
  #   1. BEFORE: Run npm test → 15 failures across 3 test suites. AFTER: Run npm test → 0 failures, all tests passing.
  #   2. AST tests show error 'At least one of --query or --file is required' because tests invoke tool without required arguments. Tests expect stub JSON response but tool now returns real tree-sitter parsing results.
  #   3. TUI tests validate specific line-by-line rendering layout (ID+Title on line 1, description on line 2, etc.) All 5 tests in suite failing suggests systematic rendering change or test expectation drift.
  #
  # ========================================
  Background: User Story
    As a developer maintaining fspec test suite
    I want to fix all failing tests identified in test run
    So that test suite passes with 100% success rate and validates all functionality correctly

  Scenario: All tests pass with 100% success rate
    Given all test failures have been fixed
    When I run "npm test"
    Then all tests should pass with 0 failures
    And the test suite should show 100% success rate

  Scenario: Delete and rewrite outdated AST research tool tests
    Given the AST research tool tests are outdated and testing stub behavior
    When I delete the outdated AST test file
    Then the AST tests should validate real tree-sitter parsing
    And I write new integration tests for actual tree-sitter parser behavior
    And all AST tests should pass with 0 failures

  Scenario: Fix TUI formatting tests by correcting implementation or expectations
    Given the TUI formatting tests are failing systematically
    When I investigate the TUI component implementation and test expectations
    Then I should fix either the implementation or the test expectations
    And I determine whether implementation regressed or test expectations are outdated
    And all TUI tests should pass with 0 failures

  Scenario: Fix or remove research integration tests based on feature existence
    Given the research integration tests are failing
    When I check if bootstrap command and CLAUDE.md generation features still exist
    Then I should either fix the tests to match current implementation
    And all research integration tests should pass with 0 failures

@done
@bug-fix
@testing
@high
@BUG-037
Feature: Fix 2 failing test suites (version-display and cli-command-registration timeout)

  """
  Test file: src/__tests__/version-display.test.ts - All tests commented out with describe.skip\nTest file: src/commands/__tests__/cli-command-registration.test.ts - Executes 93+ CLI commands sequentially using execSync\nFix approach: 1) Either uncomment and fix version-display tests OR delete the file entirely, 2) Optimize cli-command-registration by increasing timeout or using direct imports instead of execSync\nTesting: All tests must pass with 'npm test' after fixes
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All test files must contain at least one active test suite or be removed
  #   2. CLI command registration tests must complete within 30 seconds
  #   3. Tests that execute many CLI commands should use parallelization or mocking to reduce execution time
  #
  # EXAMPLES:
  #   1. version-display.test.ts has all tests commented out, causing Vitest to report 'No test suite found in file'
  #   2. cli-command-registration.test.ts executes 93+ commands sequentially with execSync, taking over 30 seconds and timing out
  #   3. After fixing version-display.test.ts, running 'npm test' should not show 'No test suite found' error
  #   4. After optimizing cli-command-registration.test.ts, the test should complete in under 30 seconds
  #
  # ========================================

  Background: User Story
    As a developer running npm test
    I want to have all tests pass successfully
    So that I can verify code quality and ensure CI/CD pipeline succeeds

  Scenario: Fix version-display.test.ts with no active tests
    Given I have a test file "src/__tests__/version-display.test.ts" with all tests commented out
    When I run "npm test"
    Then the test suite should not fail with "No test suite found in file" error
    And the test file should either contain active tests or be removed

  Scenario: Optimize cli-command-registration.test.ts to avoid timeout
    Given I have a test file "src/commands/__tests__/cli-command-registration.test.ts" that executes 93+ commands sequentially
    When I run "npm test"
    Then the test should complete in under 30 seconds
    And all command registration assertions should pass

  Scenario: Verify npm test passes after fixes
    Given I have fixed both failing test suites
    When I run "npm test"
    Then all tests should pass
    And no test timeout errors should occur
    And no "No test suite found" errors should occur

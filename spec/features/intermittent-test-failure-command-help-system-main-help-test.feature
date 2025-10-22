@done
@validation
@testing
@cli
@test-isolation
@test-infrastructure
@phase1
@BUG-025
Feature: Intermittent test failure: command-help-system main help test
  """
  Root cause: version-display.test.ts rebuilds dist/index.js during test execution, causing race condition with command-help-system.test.ts
  Solution: Mark version-display.test.ts with test.sequential() to prevent parallel execution with other CLI tests
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tests must not rebuild dist/index.js during execution as it causes race conditions
  #
  # EXAMPLES:
  #   1. version-display.test.ts runs 'npm run build' while command-help-system.test.ts executes node dist/index.js --help, causing exit code 1 or undefined
  #   2. Test passes when run in isolation but fails when run with full suite due to dist/index.js being rebuilt by version-display.test.ts
  #
  # ========================================
  Background: User Story
    As a developer running fspec test suite
    I want to run all tests concurrently without race conditions
    So that tests are reliable and pass consistently

  Scenario: Version display tests run sequentially to prevent race conditions
    Given the version-display.test.ts file contains tests that rebuild dist/index.js
    And other tests like command-help-system.test.ts execute dist/index.js
    When I run the full test suite with npm test
    Then the version-display.test.ts tests should run sequentially
    And no race condition should occur with dist/index.js access
    And all tests should pass consistently

  Scenario: Command help system test passes in full test suite
    Given the version-display.test.ts is configured to run sequentially
    When I run the full test suite including command-help-system.test.ts
    Then the command-help-system.test.ts test should complete successfully
    And the exit code should be 0
    And the test output should show "should display note about command-specific help in main help" as passed

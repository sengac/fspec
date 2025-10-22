@done
@validation
@testing
@test-isolation
@test-infrastructure
@cli
@BUG-026
Feature: Flaky tests: command-help-system and foundation-existence-check fail in full suite but pass individually
  """
  Root cause: Same as BUG-025 - version-display.test.ts rebuilding dist/index.js created race conditions with ALL CLI tests
  Solution: Fixed by BUG-025 solution - marking version-display.test.ts with describe.sequential() prevents parallel execution
  Impact: Both command-help-system.test.ts:223 and foundation-existence-check.test.ts:172 now pass consistently in full suite (verified across multiple runs)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tests that execute dist/index.js must not run in parallel with tests that rebuild it
  #
  # EXAMPLES:
  #   1. command-help-system.test.ts executes 'node dist/index.js add-question --help' and gets exit code 1 when version-display.test.ts rebuilds dist/index.js in parallel
  #   2. foundation-existence-check.test.ts executes 'fspec validate' and gets exit code undefined when version-display.test.ts rebuilds dist/index.js in parallel
  #   3. After fixing BUG-025 (marking version-display.test.ts as sequential), both tests pass consistently in full suite
  #
  # ========================================
  Background: User Story
    As a developer running full test suite
    I want to have all CLI tests pass reliably
    So that I can trust test results without intermittent failures

  Scenario: Sequential execution prevents race conditions in command-help-system test
    Given version-display.test.ts is marked with describe.sequential()
    And command-help-system.test.ts executes "node dist/index.js add-question --help"
    When I run the full test suite with npm test
    Then the command-help-system.test.ts:223 test should pass
    And the exit code should be 0
    And no race condition should occur with dist/index.js

  Scenario: Sequential execution prevents race conditions in foundation-existence-check test
    Given version-display.test.ts is marked with describe.sequential()
    And foundation-existence-check.test.ts executes "fspec validate"
    When I run the full test suite with npm test
    Then the foundation-existence-check.test.ts:172 test should pass
    And the exit code should be 0
    And no race condition should occur with dist/index.js

  Scenario: Both CLI tests pass consistently after BUG-025 fix
    Given version-display.test.ts runs sequentially instead of in parallel
    When I run the full test suite multiple times
    Then command-help-system.test.ts should pass on all runs
    And foundation-existence-check.test.ts should pass on all runs
    And no intermittent failures should occur

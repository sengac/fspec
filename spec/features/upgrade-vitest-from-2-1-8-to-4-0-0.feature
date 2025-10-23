@done
@cli
@vitest
@testing
@dependency-management
@medium
@DEP-002
Feature: Upgrade Vitest from 2.1.8 to 4.0.0

  """
  Vitest 4.0 Release Notes (Oct 22, 2025):
- Browser Mode stabilization (no longer experimental)
- Visual regression testing with toMatchScreenshot assertion
- Context import changed from @vitest/browser/context to vitest/browser
- Pool default remains 'forks' (introduced in 2.0)
- Migration guide: https://vitest.dev/blog/vitest-4
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All existing tests must pass after upgrade with no modifications
  #   2. Configuration changes must maintain safety settings (singleFork, fileParallelism: false)
  #   3. Must review official Vitest 4.0 migration guide before upgrading
  #
  # EXAMPLES:
  #   1. Run npm install -D vitest@^4.0.0 to upgrade package
  #   2. Update vitest.config.ts if breaking changes affect configuration syntax
  #   3. Run full test suite with npm test to verify compatibility
  #   4. Check npm run build to ensure TypeScript compilation still works
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we explore browser mode features in Vitest 4.0 or keep using node environment only?
  #   A: true
  #
  #   Q: Do we want to adopt visual regression testing features or defer for future work?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a fspec developer
    I want to upgrade Vitest to version 4.0 for improved stability and latest features
    So that I benefit from bug fixes, performance improvements, and maintain compatibility with the latest testing ecosystem

  Scenario: Upgrade Vitest package to version 4.0.0
    Given the project is using Vitest version 2.1.8
    And package.json lists vitest as a dev dependency
    When I run "npm install -D vitest@^4.0.0"
    Then the package.json should show vitest version ^4.0.0
    And the package-lock.json should be updated with the new version
    And node_modules should contain Vitest 4.0.0

  Scenario: Verify vitest.config.ts compatibility with version 4.0
    Given Vitest has been upgraded to version 4.0.0
    And the vitest.config.ts file contains existing configuration
    When I review the Vitest 4.0 migration guide
    Then I should identify any breaking changes affecting configuration
    And I should update vitest.config.ts to maintain safety settings
    And the pool option should remain set to "forks"
    And the singleFork option should remain true
    And the fileParallelism option should remain false

  Scenario: Run full test suite after upgrade
    Given Vitest has been upgraded to version 4.0.0
    And vitest.config.ts has been updated for compatibility
    When I run "npm test"
    Then all existing tests should pass without modification
    And no test failures should be introduced by the upgrade
    And test output should confirm Vitest 4.0.0 is being used

  Scenario: Verify TypeScript compilation after upgrade
    Given Vitest has been upgraded to version 4.0.0
    And all tests are passing
    When I run "npm run build"
    Then TypeScript compilation should complete successfully
    And no new type errors should be introduced
    And the dist/ directory should contain compiled code
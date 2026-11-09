@done
@BUG-503
@bridge
@npm
@testing
@test-infrastructure
Feature: Bridge test suite runs reproducibly
  """
  The TypeScript bridge (bridge/) keeps its vitest tests in bridge/__tests__.
  bridge/package.json declares vitest as a devDependency and a test script
  ('vitest run'), bridge/package-lock.json pins the installed tree so npm ci
  is reproducible, and bridge/vitest.config.ts runs bridge/__tests__/*.test.ts
  in the node environment. docs/TESTING.md documents the command:
  cd bridge && npm ci && npm test.
  Tests: bridge/__tests__/bridge-test-command.test.ts.
  """

  Background: User Story
    As a developer changing the TypeScript bridge
    I want to run the bridge's vitest suite with one documented command after a clean install
    So that bridge tests run the same way for everyone instead of through an ad hoc vitest install

  Scenario: The bridge package declares its test runner and test command
    Given the bridge package manifest
    When its dev dependencies and scripts are read
    Then vitest is a dev dependency
    And the test script runs 'vitest run'
    And the lock file pins the vitest version that is installed

  Scenario: The vitest config runs exactly the bridge tests in node
    Given the bridge vitest config
    When its test settings are read
    Then it includes the test files under __tests__ ending in .test.ts
    And it runs them in the node environment

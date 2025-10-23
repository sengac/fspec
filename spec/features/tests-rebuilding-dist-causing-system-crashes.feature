@cli
@test-infrastructure
@build
@testing
@critical
@bug
@BUG-036
Feature: Tests rebuilding dist causing system crashes
  """

  - Integration tests execute ./dist/index.js CLI commands via child processes
  - Tests must NOT rebuild dist/ during execution (causes race conditions and crashes)
  - Build MUST happen ONCE before test suite starts (npm run build && vitest)
  - Sequential test configuration can be removed once build is separated from test execution
  - Tests in src/commands/__tests__/ and other areas execute end-to-end CLI testing
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Integration tests in src/commands/__tests__/ and other test areas that spawn child processes to run actual fspec commands (e.g., exec('./dist/index.js validate')). These tests execute the CLI end-to-end.
  #   2. Uncertain, but likely tests running in parallel despite sequential configuration. Some tests may be calling 'npm run build' unnecessarily, which is bad practice if avoidable. The combination causes system crashes.
  #   3. Build ONCE before tests start. Change npm test script to 'npm run build && vitest'. Remove sequential test configuration since parallel builds won't be an issue anymore. Tests can still execute ./dist/index.js but never rebuild during test execution.
  #   4. Tests must NEVER call 'npm run build' during execution
  #   5. Build must happen ONCE before all tests start
  #   6. Uncertain, but likely tests running in parallel despite sequential configuration. Some tests may be calling 'npm run build' unnecessarily, which is bad practice if avoidable. The combination causes system crashes.
  #   7. Build ONCE before tests start. Change npm test script to 'npm run build && vitest'. Remove sequential test configuration since parallel builds won't be an issue anymore. Tests can still execute ./dist/index.js but never rebuild during test execution.
  #
  # EXAMPLES:
  #   1. BEFORE: npm test runs vitest directly, tests rebuild dist/ and crash. AFTER: npm test runs 'npm run build && vitest', dist/ built once, tests run safely.
  #   2. BEFORE: Sequential test config needed to prevent parallel build conflicts. AFTER: Can remove sequential config, tests run in parallel safely.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Which test files are executing CLI commands that use ./dist/index.js?
  #   A: true
  #
  #   Q: What's actually causing the crash - tests running in parallel, unnecessary npm run build calls, or both?
  #   A: true
  #
  #   Q: What's the desired fix approach?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer running tests
    I want to run tests without system crashes from parallel dist rebuilds
    So that I have reliable test execution with stable ./dist/index.js binary

  Scenario: Build once before tests to prevent crashes
    Given integration tests execute ./dist/index.js CLI commands
    And the current npm test script runs vitest directly without building first
    And tests may rebuild dist/ during execution causing crashes
    When I change the npm test script to "npm run build && vitest"
    Then dist/ is built ONCE before all tests start
    And tests execute with a stable ./dist/index.js binary
    And tests run safely without system crashes
    And no tests rebuild dist/ during execution

  Scenario: Remove sequential test configuration after build separation
    Given the build now happens once before tests start
    And dist/ is no longer rebuilt during test execution
    And sequential test configuration was added to prevent parallel build conflicts
    When I remove the sequential test configuration from vitest.config.ts
    Then tests can run in parallel safely
    And no build conflicts occur
    And test execution is faster without sequential constraints

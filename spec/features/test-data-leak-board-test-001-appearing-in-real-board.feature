@done
@high
@tui
@testing
@bug-fix
@BOARD-005
Feature: Test data leak: BOARD-TEST-001 appearing in real board
  """
  Fix requires modifying BoardView-realtime-updates.test.tsx to use mkdtemp() for temporary directories. Pattern: beforeEach creates testDir with await mkdtemp(join(tmpdir(), 'fspec-test-')), afterEach cleans up with await rm(testDir, { recursive: true, force: true }). Tests must pass testDir to loadData() method instead of using process.cwd(). Reference implementation: src/commands/__tests__/update-work-unit-ensure.test.ts shows correct pattern.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tests MUST use temporary directories created with mkdtemp() to avoid polluting real project files
  #   2. Tests MUST clean up temporary directories in afterEach() hooks using rm(dir, { recursive: true, force: true })
  #   3. Tests that write to work-units.json MUST use testDir as cwd parameter, NOT process.cwd()
  #   4. BoardView-realtime-updates.test.tsx lines 209-223 are the problematic code that writes to real spec/work-units.json
  #   5. Tests MUST restore original file state if they absolutely must modify real files (use cleanup in try-finally blocks)
  #
  # EXAMPLES:
  #   1. BoardView test creates BOARD-TEST-001 in real spec/work-units.json, causing it to appear in actual board view
  #   2. update-work-unit-ensure.test.ts correctly uses mkdtemp() to create isolated test directory in tmpdir()
  #   3. Test cleanup restores original work-units.json content at lines 260-262, but damage is already done
  #   4. Fix involves using mkdtemp() in beforeEach, passing testDir to loadData(), cleaning up in afterEach
  #
  # ========================================
  Background: User Story
    As a developer running tests
    I want to run tests without polluting real project data
    So that I can trust that tests are isolated and won't affect my actual work units

  Scenario: Test creates work unit in real project files
    Given BoardView-realtime-updates.test.tsx auto-refresh test is running
    And the test writes BOARD-TEST-001 to real spec/work-units.json at line 209-223
    When I run npm test
    Then BOARD-TEST-001 should appear in the real board view
    And this pollutes actual project data

  Scenario: Test uses isolated temporary directory
    Given update-work-unit-ensure.test.ts is a reference implementation
    And it uses mkdtemp() in beforeEach to create testDir
    When the test runs and writes to work-units.json
    Then it should write to testDir/spec/work-units.json
    And afterEach should clean up with rm(testDir, recursive, force)
    And real project files should remain untouched

  Scenario: Fix BoardView test to use temporary directory
    Given BoardView-realtime-updates.test.tsx needs fixing
    When I add mkdtemp() in beforeEach
    And pass testDir to Zustand store loadData() method
    And add afterEach cleanup with rm()
    Then the test should use isolated temp directory
    And BOARD-TEST-001 should not appear in real board
    And real spec/work-units.json should remain unchanged

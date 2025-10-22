@done
@workflow-automation
@refactoring
@hooks
@HOOK-010
Feature: Migrate hook execution to use execa library
  """
  Use execa library (same as cage project) for better process management
  Modify src/hooks/executor.ts executeHook() function
  Install execa as dependency in package.json
  Reference cage's server.ts implementation for execa usage patterns
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Hook execution must maintain timeout behavior (default 60s)
  #   2. Hook context must be passed to stdin as JSON
  #   3. stdout and stderr must be captured separately
  #   4. Hook execution results must include hookName, success, exitCode, stdout, stderr, timedOut, duration
  #   5. execa library must be installed as a dependency
  #   6. All existing hook tests must continue to pass
  #   7. Non-detached behavior - hooks run attached to fspec process
  #   8. Keep custom timeout implementation integrated with execa - use AbortController signal to cancel subprocess
  #   9. Use execa's input option for passing JSON context - it's the best practice and handles stream management automatically
  #
  # EXAMPLES:
  #   1. Hook executes successfully and returns exitCode 0 with captured stdout/stderr
  #   2. Hook times out after configured timeout period and is killed
  #   3. Hook fails with non-zero exit code and error message in stderr
  #   4. Hook receives context via stdin as JSON string
  #   5. Multiple hooks execute sequentially in order
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we support detached processes like cage does for server.ts, or keep the current non-detached behavior?
  #   A: true
  #
  #   Q: Should we use execa's built-in timeout option or keep the custom timeout implementation?
  #   A: true
  #
  #   Q: Should we use execa's input option for passing context or stick with stdin.write()?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a fspec developer
    I want to use execa for hook execution instead of child_process.spawn
    So that better process management, improved error handling, and cross-platform compatibility

  Scenario: Hook executes successfully and returns exitCode 0 with captured stdout/stderr
    Given I have a hook script that exits with code 0
    And the hook script writes "success message" to stdout
    And the hook script writes "debug info" to stderr
    When the hook is executed using execa
    Then the hook result should have success = true
    And the hook result should have exitCode = 0
    And the hook result stdout should contain "success message"
    And the hook result stderr should contain "debug info"
    And the hook result should include hookName, timedOut, and duration fields

  Scenario: Hook times out after configured timeout period and is killed
    Given I have a hook script that runs longer than the timeout period
    And the timeout is configured to 2 seconds
    When the hook is executed using execa with AbortController
    Then the hook should be killed after 2 seconds
    And the hook result should have timedOut = true
    And the hook result should have exitCode = null
    And the hook result should have success = false

  Scenario: Hook fails with non-zero exit code and error message in stderr
    Given I have a hook script that exits with code 1
    And the hook script writes "error occurred" to stderr
    When the hook is executed using execa
    Then the hook result should have success = false
    And the hook result should have exitCode = 1
    And the hook result stderr should contain "error occurred"

  Scenario: Hook receives context via stdin as JSON string
    Given I have a hook script that reads from stdin
    And I have a hook context with workUnitId "HOOK-010"
    When the hook is executed using execa with input option
    Then the hook should receive the context as JSON via stdin
    And the hook should be able to parse the workUnitId from stdin

  Scenario: Multiple hooks execute sequentially in order
    Given I have three hooks configured: "hook-a", "hook-b", and "hook-c"
    When all hooks are executed using executeHooks function
    Then "hook-a" should complete before "hook-b" starts
    And "hook-b" should complete before "hook-c" starts
    And all hook results should be returned in execution order

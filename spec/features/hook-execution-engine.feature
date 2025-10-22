@done
@workflow-automation
@hooks
@HOOK-003
Feature: Hook execution engine

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # USER STORY:
  #   As a fspec developer
  #   I want to execute hook scripts at lifecycle events
  #   So that I can run custom automation, quality gates, and notifications during fspec commands
  #
  # BUSINESS RULES:
  #   1. Hook execution engine spawns child processes for each hook script
  #   2. Hook context is passed as JSON to stdin (workUnitId, event, timestamp, etc.)
  #   3. stdout and stderr are captured and displayed to user/AI agent
  #   4. Exit code 0 indicates success, non-zero indicates failure
  #   5. Hooks timeout after configured duration (default 60s)
  #   6. Blocking hooks halt command execution on failure (non-zero exit)
  #   7. Hooks execute sequentially in the order defined in config
  #   8. Hook processes inherit parent environment variables
  #
  # EXAMPLES:
  #   1. Execute single non-blocking hook that succeeds (exit 0) - output shown, command continues
  #   2. Execute single non-blocking hook that fails (exit 1) - warning shown, command continues
  #   3. Execute blocking hook that fails - command halted, error returned to user/AI
  #   4. Hook times out after 60s - process killed, timeout error shown
  #   5. Hook receives context via stdin: {"workUnitId":"AUTH-001","event":"post-implementing","timestamp":"2025-01-15T10:30:00Z"}
  #   6. Multiple hooks execute sequentially - first runs to completion, then second starts
  #   7. Hook stdout/stderr captured and displayed: 'Running tests...' visible to user
  #
  # ========================================
  Background: User Story
    As a fspec developer
    I want to execute hook scripts at lifecycle events
    So that I can run custom automation, quality gates, and notifications during fspec commands

  Scenario: Execute single non-blocking hook that succeeds
    Given I have a hook configuration with a non-blocking hook "lint" for "post-implementing"
    And the hook script "spec/hooks/lint.sh" exits with code 0
    When I execute the hook for event "post-implementing"
    Then the hook should be spawned as a child process
    And the hook stdout should be captured and displayed
    And the hook should complete successfully
    And the command should continue execution

  Scenario: Execute single non-blocking hook that fails
    Given I have a hook configuration with a non-blocking hook "check" for "post-testing"
    And the hook script "spec/hooks/check.sh" exits with code 1
    When I execute the hook for event "post-testing"
    Then the hook should be spawned as a child process
    And a warning should be displayed about the hook failure
    And the command should continue execution despite the failure

  Scenario: Execute blocking hook that fails
    Given I have a hook configuration with a blocking hook "validate" for "post-implementing"
    And the hook script "spec/hooks/validate.sh" exits with code 1
    When I execute the hook for event "post-implementing"
    Then the hook should be spawned as a child process
    And an error should be displayed about the hook failure
    And the command execution should be halted
    And the fspec command should exit with non-zero code

  Scenario: Hook times out after configured duration
    Given I have a hook configuration with a hook "slow-test" with timeout 2 seconds
    And the hook script "spec/hooks/slow-test.sh" runs for 10 seconds
    When I execute the hook for event "post-testing"
    Then the hook process should be killed after 2 seconds
    And a timeout error should be displayed
    And the hook should be marked as failed

  Scenario: Hook receives context via stdin
    Given I have a hook configuration with a hook "notify" for "post-implementing"
    And I am executing in context of work unit "AUTH-001"
    When I execute the hook for event "post-implementing"
    Then the hook should receive JSON context via stdin
    And the context should include "workUnitId" field with value "AUTH-001"
    And the context should include "event" field with value "post-implementing"
    And the context should include "timestamp" field with ISO 8601 format

  Scenario: Multiple hooks execute sequentially
    Given I have a hook configuration with hooks "first" and "second" for "post-implementing"
    And both hooks are configured to execute in order
    When I execute the hooks for event "post-implementing"
    Then hook "first" should start execution
    And hook "first" should complete before hook "second" starts
    And hook "second" should start execution after hook "first" completes
    And hooks should execute in sequential order

  Scenario: Hook stdout and stderr are captured and displayed
    Given I have a hook configuration with a hook "test" for "post-testing"
    And the hook script outputs "Running tests..." to stdout
    And the hook script outputs "Warning: deprecated API" to stderr
    When I execute the hook for event "post-testing"
    Then the stdout message "Running tests..." should be captured
    And the stderr message "Warning: deprecated API" should be captured
    And both stdout and stderr should be displayed to the user

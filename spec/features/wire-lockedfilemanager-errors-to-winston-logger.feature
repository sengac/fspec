@phase-1
@file-ops
@infrastructure
@logging
@LOG-002
Feature: Wire LockedFileManager errors to winston logger
  """
  Integrates winston logger (LOG-001) with LockedFileManager (LOCK-002). Replaces console.error/console.log with logger.error/logger.debug. Existing logMetrics() function routes through winston logger.debug() when FSPEC_DEBUG_LOCKS=1. Lock errors (timeouts, compromises, parse failures) logged at error level with structured context (file path, lock type, error details). Import logger from src/utils/logger.ts into src/utils/file-manager.ts.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Replace all console.error/console.log calls in file-manager.ts with logger.error/logger.debug
  #   2. Log lock errors with context: file path, lock type (READ/WRITE), error message
  #   3. Log lock metrics (wait time, hold duration, retries) at debug level when FSPEC_DEBUG_LOCKS enabled
  #   4. Log lock compromises at error level with file path and details
  #   5. Continue using existing logMetrics() function but route through winston instead of console.log
  #
  # EXAMPLES:
  #   1. Lock timeout occurs - logs 'Lock timeout for work-units.json (WRITE lock) after 10s' to winston at error level
  #   2. Lock compromised during transaction - logs 'Lock compromised for epics.json' to winston at error level with stack trace
  #   3. FSPEC_DEBUG_LOCKS=1 enabled - logs lock metrics 'Acquired READ lock on work-units.json (waited 50ms, held 100ms, retries 0)' at debug level
  #   4. JSON parse error in readJSON - logs 'Failed to parse work-units.json: Unexpected token' at error level
  #
  # ========================================
  Background: User Story
    As a fspec developer
    I want to have file locking errors logged to winston
    So that I can debug concurrency issues and lock failures in production

  Scenario: Log lock timeout error
    Given LockedFileManager is configured to use winston logger
    And a lock timeout occurs for work-units.json with WRITE lock
    When the timeout error is thrown
    Then winston should log at error level
    And the log message should contain the file path "work-units.json"
    And the log message should contain the lock type "WRITE"
    And the log message should contain "timeout"
    And the log should be appended to ~/.fspec/fspec.log

  Scenario: Log lock compromised error
    Given LockedFileManager is configured to use winston logger
    And a lock compromise occurs for epics.json
    When the onCompromised callback is triggered
    Then winston should log at error level
    And the log message should contain "Lock compromised"
    And the log message should contain the file path "epics.json"
    And the log should include error details
    And the log should be appended to ~/.fspec/fspec.log

  Scenario: Log lock metrics in debug mode
    Given LockedFileManager is configured to use winston logger
    And FSPEC_DEBUG_LOCKS environment variable is set to "1"
    And a READ lock is acquired on work-units.json
    When logMetrics is called with wait time 50ms, hold duration 100ms, retries 0
    Then winston should log at debug level
    And the log message should contain "Acquired READ lock"
    And the log message should contain "work-units.json"
    And the log message should contain "waited 50ms"
    And the log message should contain "held 100ms"
    And the log message should contain "retries 0"

  Scenario: Log JSON parse errors
    Given LockedFileManager is configured to use winston logger
    And readJSON encounters a JSON parse error in work-units.json
    When the parse error is caught
    Then winston should log at error level
    And the log message should contain "Failed to parse"
    And the log message should contain "work-units.json"
    And the log message should contain the parse error details
    And the log should be appended to ~/.fspec/fspec.log

  Scenario: Replace console calls with winston
    Given file-manager.ts uses console.error and console.log
    When LockedFileManager is refactored to use winston
    Then all console.error calls should be replaced with logger.error
    And all console.log debug calls should be replaced with logger.debug
    And no console.error or console.log calls should remain in file-manager.ts

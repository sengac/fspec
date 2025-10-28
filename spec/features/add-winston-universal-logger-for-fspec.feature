@done
@file-ops
@phase-1
@infrastructure
@logging
@cross-platform
@LOG-001
Feature: Add winston universal logger for fspec
  """
  Uses winston library for universal logging. File transport configured with append-only mode using fs.createWriteStream flags. Singleton pattern for logger instance exported from src/utils/logger.ts. Log file path uses os.homedir() + path.join() for cross-platform compatibility (Windows: C:\Users\<user>\.fspec\fspec.log, macOS/Linux: /Users|home/<user>/.fspec/fspec.log). Winston's native stream handling provides safe concurrent writes without requiring LockedFileManager.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Log file must be stored in ~/.fspec/fspec.log (platform-agnostic home directory)
  #   2. Winston must use append-only file transport (safe for concurrent writes)
  #   3. Support log levels: error, warn, info, debug
  #   4. Replace all console.error calls with logger.error throughout codebase
  #   5. Provide singleton logger instance importable throughout codebase
  #
  # EXAMPLES:
  #   1. Developer imports logger and calls logger.error('message') - logs to ~/.fspec/fspec.log
  #   2. Multiple fspec processes run concurrently - all append to same log file without corruption
  #   3. Developer calls logger.debug('debug message') - logged only if debug level enabled
  #
  # ========================================
  Background: User Story
    As a fspec developer
    I want to have a universal logging system with winston
    So that I can debug issues, track command execution, and maintain consistent logging across the codebase

  Scenario: Log error message to file
    Given winston logger is configured with file transport
    And the log file path is in the user's home directory at .fspec/fspec.log
    When I import the logger singleton
    And I call logger.error('test error message')
    Then the message should be appended to the log file
    And the log entry should contain timestamp and level 'error'
    And the log file should exist at the correct path on any platform

  Scenario: Concurrent writes from multiple processes
    Given winston logger is configured with append-only file transport
    And multiple fspec processes are running concurrently
    When each process calls logger.error() simultaneously
    Then all log messages should be appended without corruption
    And no log entries should be lost or interleaved

  Scenario: Debug level logging
    Given winston logger is configured with log level 'info'
    When I call logger.debug('debug message')
    Then the debug message should not be logged
    When I configure log level to 'debug'
    And I call logger.debug('debug message')
    Then the debug message should be appended to the log file

  Scenario: Cross-platform log file path resolution
    Given I am running fspec on Windows
    When winston logger initializes
    Then the log file path should resolve to C:\Users\<username>\.fspec\fspec.log
    Given I am running fspec on macOS
    When winston logger initializes
    Then the log file path should resolve to /Users/<username>/.fspec/fspec.log
    Given I am running fspec on Linux
    When winston logger initializes
    Then the log file path should resolve to /home/<username>/.fspec/fspec.log

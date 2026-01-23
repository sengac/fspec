@done
@infrastructure
@utils
@LOG-003
Feature: Capture all console methods and redirect to winston logger
  """
  Depends on existing winston logger from src/utils/logger.ts (LOG-001)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Console capture must be initialized at application startup before any other imports that use console methods
  #   2. All console methods must be captured: log, error, warn, info, debug, trace
  #   3. Original console output must be preserved (still appear in terminal)
  #   4. ANSI escape codes (chalk formatting) must be stripped from log file output
  #   5. Console methods must map to appropriate winston log levels: log->info, error->error, warn->warn, info->info, debug->debug, trace->debug
  #   6. Multiple console arguments must be joined with spaces
  #   7. Objects must be serialized to JSON for logging
  #   8. Error objects must include stack trace when available
  #   9. A restoreConsole function must be provided for test isolation
  #
  # EXAMPLES:
  #   1. console.log('hello') outputs 'hello' to terminal AND logs '[info]: hello' to ~/.fspec/fspec.log
  #   2. console.error('failed') outputs 'failed' to terminal AND logs '[error]: failed' to ~/.fspec/fspec.log
  #   3. console.warn('deprecated') outputs 'deprecated' to terminal AND logs '[warn]: deprecated' to ~/.fspec/fspec.log
  #   4. console.log with chalk formatting like '\x1b[31mred\x1b[0m' shows 'red' colored in terminal but logs plain 'red' without escape codes in log file
  #   5. console.log('hello', 'world', 123) joins arguments and logs '[info]: hello world 123'
  #   6. console.log({foo: 'bar'}) serializes the object and logs '[info]: {"foo": "bar"}'
  #   7. console.error(new Error('test')) includes the stack trace in the log file
  #   8. console.trace('marker') includes both the message and stack trace in the log file at debug level
  #   9. Running 'fspec list-features' shows output in terminal and the same output (without colors) appears in ~/.fspec/fspec.log
  #   10. After calling restoreConsole() in a test, console methods no longer log to winston
  #
  # ========================================
  Background: User Story
    As a developer debugging fspec
    I want to have all console output captured to the log file
    So that I can review historical console output without needing to be present when issues occur

  Scenario: Capture console.log to info level
    Given console capture has been initialized
    When console.log is called with 'hello'
    Then 'hello' is output to the terminal
    And the log file contains '[info]: hello'

  Scenario: Capture console.error to error level
    Given console capture has been initialized
    When console.error is called with 'failed'
    Then 'failed' is output to the terminal
    And the log file contains '[error]: failed'

  Scenario: Capture console.warn to warn level
    Given console capture has been initialized
    When console.warn is called with 'deprecated'
    Then 'deprecated' is output to the terminal
    And the log file contains '[warn]: deprecated'

  Scenario: Strip ANSI escape codes from log file
    Given console capture has been initialized
    When console.log is called with chalk-formatted text containing ANSI codes
    Then the terminal shows the colored text
    And the log file contains plain text without ANSI escape codes

  Scenario: Join multiple arguments with spaces
    Given console capture has been initialized
    When console.log is called with 'hello', 'world', and 123
    Then the log file contains '[info]: hello world 123'

  Scenario: Serialize objects to JSON
    Given console capture has been initialized
    When console.log is called with an object {foo: 'bar'}
    Then the log file contains the JSON serialized object

  Scenario: Include stack trace for Error objects
    Given console capture has been initialized
    When console.error is called with an Error object
    Then the log file contains the error message and stack trace

  Scenario: Capture console.trace with stack trace at debug level
    Given console capture has been initialized
    When console.trace is called with 'marker'
    Then the log file contains '[debug]: marker' with a stack trace

  Scenario: Restore console methods for test isolation
    Given console capture has been initialized
    When restoreConsole is called
    Then subsequent console calls no longer log to winston
    And console output still appears in the terminal

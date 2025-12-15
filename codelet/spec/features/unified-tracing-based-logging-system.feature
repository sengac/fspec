@cli-interface
@scaffold
@logging
@infrastructure
@tracing
@critical
@CORE-007
Feature: Unified Tracing-Based Logging System

  """
  Architecture notes:
  - Rust port of codelet's Winston logging system using tracing + tracing-subscriber + tracing-appender
  - Creates dedicated logging module (src/logging/mod.rs) with init_logging() function
  - Logs to ~/.codelet/logs/ with daily rotation (tracing_appender::rolling::daily)
  - Uses JSON format for machine parsing (tracing-subscriber's json layer)
  - File-only logging (NO stdout/stderr) to avoid CLI interference
  - Supports RUST_LOG env var and --verbose CLI flag for debug mode

  Implementation details:
  - Replace all diagnostic eprintln! calls with tracing macros: claude.rs:314 eprintln -> warn!, cli.rs:124 eprintln -> error!
  - Add structured logging with metadata: error!(stop_reason = %other, "message")
  - Preserve user-facing println! calls (cli.rs:71, cli.rs:85) - they show config/messages, not diagnostics
  - Add logging to RigAgent::prompt() and RigAgent::prompt_streaming() for agent execution tracking
  - Add debug! logging to all 7 tool execute() methods (Read, Write, Bash, Grep, Glob, Edit, AstGrep) with tool name and input parameters
  - Follows codelet pattern: logger.debug('Executing tool', { toolName, toolInput })
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Logging must support multiple log levels: error, warn, info, debug, trace
  #   2. Logs must be written to files in ~/.codelet/logs/ directory (not stdout) to avoid interfering with CLI output
  #   3. Log files must use daily rotation (date-based filenames like codelet-2024-12-02.log)
  #   4. Logs must retain last 5 files and enforce 10MB max file size before rotation
  #   5. Logging must support structured metadata alongside messages (like Winston's metadata parameter)
  #   6. Debug mode must be enabled via RUST_LOG env variable (following Rust conventions) or --verbose CLI flag
  #   7. All println! and eprintln! calls must be replaced with appropriate tracing macros (info!, debug!, error!, warn!)
  #   8. Log format must use JSON for machine parsing (tracing-subscriber's json layer)
  #
  # EXAMPLES:
  #   1. Initialize logging in main.rs with tracing_subscriber, setting up JSON file output to ~/.codelet/logs/ with daily rotation
  #   2. Replace eprintln!("Unknown stop_reason: {}", other) in claude.rs:314 with warn!(stop_reason = %other, "Unknown stop_reason from Anthropic API")
  #   3. Replace eprintln!("Error: {}", e) in cli.rs:124 with error!(error = %e, "Agent execution failed")
  #   4. Add info! logging in RigAgent::prompt() and RigAgent::prompt_streaming() to track agent execution start/completion
  #   5. Add debug\! logging for tool execution in each tool's execute() method (Read, Write, Bash, Grep, Glob, Edit, AstGrep)
  #   6. Running RUST_LOG=debug codelet "prompt" should enable debug-level logging to file while keeping stdout clean
  #   7. Log files should be named: codelet-2024-12-02.log, codelet-2024-12-03.log with JSON format {"timestamp":"...","level":"INFO","message":"...","fields":{...}}
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we keep the current simple tracing_subscriber::fmt() setup in main.rs, or implement a full logger module (like codelet's logger.ts) for easier initialization and configuration?
  #   A: Create a dedicated logging module (src/logging/mod.rs) with init_logging() function for clean initialization, similar to codelet's logger.ts pattern. This encapsulates configuration and makes testing easier.
  #
  #   Q: Should stdout remain completely clean (only CLI output), or allow colored console logging in addition to file logging (similar to Winston's console transport)?
  #   A: Keep stdout completely clean (file-only logging). CLI output should only show user-facing information. This matches codelet's approach and avoids interference with ratatui/crossterm UI rendering.
  #
  #   Q: Should we add tracing spans for performance tracking (like codelet's token tracking), or keep it simple with just event logging (info\!, debug\!, etc.)?
  #   A: Keep it simple with event logging only (info!, debug!, error!, warn!) for Phase 1. Tracing spans can be added later as CORE-008 if performance tracking becomes a priority.
  #
  #   Q: User-facing println\! calls (like cli.rs:71 config path, cli.rs:85 'Interactive mode not yet implemented') should remain as println\!, or convert to a different output mechanism?
  #   A: User-facing println! calls (config path display, user messages) should remain as println!. Only convert diagnostic/debug eprintln! calls to tracing macros. Separate concern: user output vs system logging.
  #
  # ========================================

  Background: User Story
    As a developer debugging codelet
    I want to have comprehensive structured logging with file rotation
    So that I can diagnose issues, track execution flow, and maintain audit trails without interfering with stdout/stderr

  Scenario: Initialize logging module with daily file rotation
    Given I create src/logging/mod.rs with init_logging() function
    And I configure tracing_subscriber with JSON formatter
    And I set up tracing_appender::rolling::daily to ~/.codelet/logs/
    When the logging system initializes in main.rs
    Then logs should be written to ~/.codelet/logs/codelet-YYYY-MM-DD.log
    And log files should use daily rotation
    And log files should retain last 5 files
    And log files should enforce 10MB max file size before rotation
    And log format should be JSON with timestamp, level, message, and fields

  Scenario: Replace eprintln with warn! macro for unknown stop_reason
    Given I have eprintln!("Unknown stop_reason: {}", other) in claude.rs:314
    When I replace it with warn!(stop_reason = %other, "Unknown stop_reason from Anthropic API")
    Then the warning should be logged to file with structured metadata
    And stdout should remain clean (no console output)

  Scenario: Replace eprintln with error! macro for agent execution failures
    Given I have eprintln!("Error: {}", e) in cli.rs:124
    When I replace it with error!(error = %e, "Agent execution failed")
    Then the error should be logged to file with structured metadata
    And stdout should remain clean (no console output)

  Scenario: Add info logging to RigAgent for execution tracking
    Given I am in RigAgent::prompt() method
    When I add info!(prompt = %prompt, "Starting agent execution") at method start
    And I add info!(response_length = response.len(), "Agent execution completed") at method end
    Then agent execution should be logged with prompt and response metadata
    And the same pattern should apply to RigAgent::prompt_streaming()

  Scenario: Add debug logging to all tool execute() methods
    Given I am implementing tool execution in Read, Write, Bash, Grep, Glob, Edit, and AstGrep
    When I add debug!(tool = "Read", input = ?args, "Executing tool") at the start of each execute() method
    Then tool executions should be logged with tool name and input parameters
    And debug logs should only appear when RUST_LOG=debug is set

  Scenario: Enable debug mode via RUST_LOG environment variable
    Given I have the logging system initialized
    When I run RUST_LOG=debug codelet "list files"
    Then debug-level logs should be written to the log file
    And stdout should only show the CLI output (clean, no log messages)

  Scenario: Enable debug mode via --verbose CLI flag
    Given I have the logging system initialized
    When I run codelet --verbose "list files"
    Then debug-level logs should be written to the log file
    And the --verbose flag should set RUST_LOG=debug internally
    And stdout should only show the CLI output (clean, no log messages)

  Scenario: Log files use JSON format with structured fields
    Given I have the logging system configured
    When I log an error with metadata: error!(user_id = 42, "Authentication failed")
    Then the log file should contain a JSON entry like:
      """
      {"timestamp":"2024-12-02T10:30:00.123Z","level":"ERROR","target":"codelet::auth","fields":{"user_id":42},"message":"Authentication failed"}
      """
    And the JSON should be machine-parseable for analysis tools

  Scenario: Preserve user-facing println! calls
    Given I have println! calls for user messages in cli.rs:71 and cli.rs:85
    When I implement the logging system
    Then user-facing println! calls should remain unchanged
    And only diagnostic eprintln! calls should be converted to tracing macros
    And user output (config path, "Interactive mode not yet implemented") should go to stdout

  Scenario: Log rotation retains only last 5 files
    Given I have daily log rotation configured
    When 7 days pass and 7 log files are created
    Then only the last 5 log files should be retained
    And older log files should be automatically deleted
    And log files should be named: codelet-2024-12-01.log, codelet-2024-12-02.log, etc.

@done
@CLI-022
Feature: Debug capture system for LLM session diagnostics

  """
  Reuse src/logging/mod.rs tracing infrastructure for structured logging
  Reuse dirs::home_dir() pattern from logging/mod.rs for ~/.codelet/debug/ path
  Handle /debug command in src/cli/interactive.rs similar to provider switch pattern at line ~96
  Reuse session.token_tracker, session.turns, session.messages for capturing session data
  Reuse ProviderManager API for provider/model metadata: current_provider_name(), context_window()
  Create new src/debug_capture.rs module as singleton manager (mirrors codelet's debug-capture.ts)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The /debug command toggles debug capture on and off
  #   2. When enabled, a new session file is created in ~/.codelet/debug/ with JSONL format
  #   3. Session files use naming pattern: session-YYYY-MM-DDTHH-mm-ss.jsonl
  #   4. A symlink 'latest.jsonl' always points to the most recent session file
  #   5. Debug directory is created with 0o700 permissions (owner read/write/execute only)
  #   6. Sensitive headers (authorization, x-api-key, anthropic-api-key, openai-api-key, api-key) are automatically redacted to [REDACTED]
  #   7. When debug is disabled, a summary report is generated as session-*.summary.md
  #   8. Debug capture has zero overhead when disabled - all capture calls short-circuit immediately
  #   9. Events are captured with timestamp, sequence number, event type, turn ID, and optional request ID for correlation
  #   10. The /debug command is a protected built-in command that cannot be overridden by custom commands
  #   11. Supported event types: session.start, session.end, api.request, api.response.start, api.response.chunk, api.response.end, api.error, tool.call, tool.result, tool.error, context.update, token.update, compaction.triggered, provider.switch, log.entry, user.input, command.executed
  #   12. Session metadata includes provider name and model name set when debug is enabled
  #   13. Implementation must reuse existing codelet infrastructure (logging, config, file I/O patterns) following DRY/SOLID principles
  #   14. This is an EXACT one-for-one port of codelet's /debug feature - no modifications, no shortcuts, no stubs
  #
  # EXAMPLES:
  #   1. User types /debug when disabled -> debug capture starts, returns 'Debug capture enabled. Session file: ~/.codelet/debug/session-2024-01-15T10-30-00.jsonl'
  #   2. User types /debug when enabled -> debug capture stops, generates summary, returns 'Debug capture disabled. Session saved to ~/.codelet/debug/session-2024-01-15T10-30-00.jsonl'
  #   3. API request with Authorization header -> header value replaced with [REDACTED] in captured event
  #   4. User makes 5 conversation turns with tool calls -> session file contains all events with sequential sequence numbers and correlated request IDs
  #   5. Debug directory doesn't exist -> directory created with 0o700 permissions before writing session file
  #   6. Session ends after 5 turns with 47 events -> summary.md shows session stats: 5 turns, 47 events, duration, provider/model info
  #   7. Debug disabled, code calls capture() function -> function returns immediately without any I/O
  #   8. Tool execution: Bash 'ls -la' -> captures tool.call event with tool name and args, then tool.result event with output
  #   9. Multiple sessions in same day -> latest.jsonl symlink updated to point to most recent session file
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the debug directory use ~/.codelet/debug/ (like codelet) or ~/.codelet/debug/ to match the Rust project naming?
  #   A: Use ~/.codelet/debug/ exactly as in the original codelet implementation - one for one copy
  #
  #   Q: The codelet implementation has 17 event types - should we implement all of them in the initial port, or start with a subset of the most critical ones?
  #   A: Implement ALL 17 event types exactly as in codelet - one for one copy, no subset
  #
  #   Q: Should the summary report format (markdown) be identical to codelet, or are there improvements you'd like to make?
  #   A: Summary report format must be identical to codelet - one for one copy, no improvements
  #
  # ========================================

  Background: User Story
    As a developer debugging agent issues
    I want to capture complete session context including API requests, tool executions, and logs
    So that I can diagnose agent crashes and unexpected behaviors using LLM analysis

  # ========================================
  # SCENARIOS
  # ========================================
  @toggle
  @cli
  Scenario: Enable debug capture with /debug command
    Given the agent is running
    And debug capture is disabled
    When I enter the "/debug" command
    Then I should see "Debug capture started"
    And I should see the path to the debug session file
    And the debug directory "~/.codelet/debug/" should exist
    And a new JSONL session file should be created

  @toggle
  @cli
  Scenario: Disable debug capture with /debug command
    Given the agent is running
    And debug capture is enabled with an active session
    When I enter the "/debug" command
    Then I should see "Debug capture stopped"
    And I should see the path to the saved session file
    And the session file should be closed and complete

  @api
  @capture
  Scenario: Capture API request with correlation ID
    Given debug capture is enabled
    When the agent makes an LLM API request
    Then an "api.request" event should be written to the debug stream
    And the event should contain a unique "requestId"
    And the event should contain the request headers with credentials redacted
    And the event should contain the full request payload
    And the event should contain a timestamp

  @api
  @capture
  Scenario: Capture API response with correlation ID
    Given debug capture is enabled
    And an API request was made with requestId "req-123"
    When the LLM API returns a response
    Then an "api.response.end" event should be written to the debug stream
    And the event should contain the same "requestId" as the request
    And the event should contain the response duration in milliseconds
    And the event should contain token usage information

  @tool
  @capture
  Scenario: Capture tool call with arguments
    Given debug capture is enabled
    When the agent executes a tool call
    Then a "tool.call" event should be written to the debug stream
    And the event should contain the tool name
    And the event should contain the tool arguments
    And the event should contain a unique "toolCallId"

  @tool
  @capture
  Scenario: Capture tool result with timing
    Given debug capture is enabled
    And a tool call was made with toolCallId "tool-456"
    When the tool execution completes
    Then a "tool.result" event should be written to the debug stream
    And the event should contain the same "toolCallId"
    And the event should contain the execution duration in milliseconds
    And the event should contain the exit code for bash tools

  @logging
  @integration
  Scenario: Merge tracing log entries into debug stream
    Given debug capture is enabled
    When the application logs an info message via tracing
    Then a "log.entry" event should be written to the debug stream
    And the event should contain the log level "info"
    And the event should contain the log message
    And the event should have a timestamp synchronized with other events

  @summary
  @output
  Scenario: Generate session summary on capture stop
    Given debug capture is enabled
    And the session has recorded multiple events
    When I stop debug capture with "/debug" command
    Then a summary markdown file should be generated
    And the summary should contain session statistics
    And the summary should contain an event timeline
    And the summary should list any errors or warnings

  @security
  @redaction
  Scenario: Redact sensitive credentials from captured headers
    Given debug capture is enabled
    When the agent makes an API request with an authorization header
    Then the captured "api.request" event should have headers
    And the "authorization" header value should be "[REDACTED]"
    And the "x-api-key" header value should be "[REDACTED]" if present

  @performance
  @disabled
  Scenario: Zero overhead when debug capture is disabled
    Given debug capture is disabled
    When the agent processes multiple LLM requests and tool calls
    Then no debug events should be written
    And no debug files should be created
    And the DebugCaptureManager should short-circuit all capture calls

  @directory
  @cross-platform
  Scenario: Create debug directory with secure permissions
    Given the debug directory does not exist
    When debug capture is enabled for the first time
    Then the directory "~/.codelet/debug/" should be created
    And the directory should have permissions 0o700
    And the path should resolve correctly on macOS, Linux, and Windows

  @session
  @metadata
  Scenario: Record session start metadata
    Given the agent is running with provider "claude" and model "claude-sonnet-4"
    When debug capture is enabled
    Then a "session.start" event should be written
    And the event should contain the provider name
    And the event should contain the model name
    And the event should contain environment information
    And the event should contain the context window size
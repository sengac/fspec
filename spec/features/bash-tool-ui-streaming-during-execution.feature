@done
@tool-execution
@codelet
@high
@TOOL-011
Feature: Bash Tool UI Streaming During Execution
  """
  Implementation approach based on research of opencode, aider, gemini-cli, letta. Use tokio spawn with piped stdout, read lines in loop, emit via StreamOutput trait. See attached research document for detailed comparison.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Output must stream to UI in real-time as subprocess writes to stdout/stderr
  #   2. LLM must still receive complete buffered output after command completion (not streamed chunks)
  #   3. Output truncation limits (MAX_OUTPUT_CHARS) must still apply to final LLM result
  #   4. Streaming must work through existing StreamOutput trait callbacks
  #   5. Timeout handling must continue to work correctly with streaming
  #
  # EXAMPLES:
  #   1. User runs 'npm install' - sees package download progress line-by-line in TUI as it happens
  #   2. User runs 'cargo build' - sees compilation progress streaming in real-time, then LLM receives full output
  #   3. Command produces 50KB output - user sees it streaming, but LLM result is truncated to MAX_OUTPUT_CHARS limit
  #   4. Command times out after 120s - user saw partial output during execution, error returned to LLM
  #   5. Similar to opencode pattern: emit_tool_progress callback called on each stdout chunk
  #
  # ========================================
  Background: User Story
    As a developer using codelet CLI
    I want to see bash command output streaming in real-time during execution
    So that I get visual feedback that commands are running and can see progress without waiting for completion

  @streaming
  @happy-path
  Scenario: Stream command output to UI in real-time
    Given a bash command that produces incremental output
    When the command executes through the bash tool
    Then output chunks should be emitted to the UI as they are produced
    And the user should see output appearing progressively

  @streaming
  @llm-integration
  Scenario: Buffer complete output for LLM response
    Given a bash command that produces multiple lines of output
    When the command completes execution
    Then the LLM should receive the complete buffered output
    And the output should not be sent as individual streaming chunks to the LLM

  @truncation
  @edge-case
  Scenario: Truncate large output for LLM while streaming full output to UI
    Given a bash command that produces output exceeding MAX_OUTPUT_CHARS
    When the command executes and completes
    Then the UI should see all output streamed in real-time
    And the LLM result should be truncated to MAX_OUTPUT_CHARS limit
    And a truncation warning should be included in the LLM result

  @error-handling
  Scenario: Handle abort (ESC) with partial streamed output
    Given a bash command that streams some output and then keeps running
    When the user presses ESC and the bash abort is requested during execution
    Then the user should have seen the partial output streamed before the abort
    And the LLM should receive a "Command interrupted by user" error
    And the command should not run to completion

  @integration
  @callback
  Scenario: Emit progress through StreamOutput trait
    Given a bash command is executing
    When stdout chunks are received from the subprocess
    Then each stdout line should trigger a stream callback
    And the callback should receive each output line as it is produced

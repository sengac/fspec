@done
@tool-execution
@session
@tool-integration
@tools
@RPC-398
Feature: Bash/tool output does not stream incrementally in Rust TUI (session-id key mismatch)

  """
  Root cause: tool-progress registry (codelet/tools/src/session_registry.rs:95-100) uses exact HashMap lookup with no fallback. Registration was under Uuid::nil() (cli/src/interactive/stream_loop.rs:459-462, BUG-126) while BashTool emits under the real session.id (bash.rs:66,268 -> bash_streams.rs:73).
  Fix: thread the real per-session UUID (the one passed to create_rig_agent/BashTool::new) into the stream-loop registration site so set_tool_progress_callback(session_id, ...) uses the same id BashTool emits with. Clear with the same id.
  Constraint: run_agent_stream_internal receives &mut Session (codelet_cli::session::Session) which has NO id field; the real UUID must be passed down from the caller (agent_runner.rs:32 / background loop) that also builds BashTool. Preserve BUG-126 exact-match isolation (no global fallback).
  Regression test must exercise the real path: register a callback under a real UUID S, run a BashTool::new(S) via Tool::call, and assert ToolProgress chunks arrive (before ToolResult). Existing tests only used Uuid::nil()+call_with_streaming, missing the registration/emit key agreement.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The tool-progress callback MUST be registered under the same session UUID that BashTool emits with (the real per-session id passed to create_rig_agent/BashTool::new), not Uuid::nil()
  #   2. Incremental ToolProgress chunks MUST be delivered while the command is still running, not only at completion via the ToolResult path
  #   3. Callback registration MUST be cleared using the same session id when the stream ends, preserving BUG-126 session isolation (no cross-session bleed, no leak)
  #   4. The registry lookup remains an exact per-session match (no global fallback) so a session only receives its own tool progress
  #
  # EXAMPLES:
  #   1. A BashTool built with session id S emits ToolProgress under S; a callback registered under S receives each line as it is produced
  #   2. A callback registered under session A does NOT receive progress emitted by a BashTool built with session B (isolation preserved)
  #   3. Running a bash command that prints 3 lines with delays yields 3 incremental ToolProgress deliveries before the final ToolResult
  #   4. After the stream loop clears the callback for session S, a subsequent emit under S is a no-op (registration removed)
  #
  # ========================================

  Background: User Story
    As a fspec-tui user
    I want to see bash/tool output stream line-by-line while a command is running
    So that I get live feedback during long-running tools instead of a frozen card until completion

  Scenario: Progress reaches a callback registered under the same session id the tool emits with
    Given a tool-progress callback is registered under a real session id S
    And a BashTool is built with the same session id S
    When the BashTool runs a command that produces output lines
    Then the callback registered under S receives each output line as it is produced

  Scenario: Session isolation - a callback does not receive another session's progress
    Given a tool-progress callback is registered under session id A
    And a BashTool is built with a different session id B
    When the BashTool built with session id B runs a command that produces output
    Then the callback registered under session id A receives no progress

  Scenario: Incremental progress is delivered while the command is still running
    Given a tool-progress callback is registered under a real session id S
    And a BashTool is built with the same session id S
    When the BashTool runs a command that prints three lines with delays between them
    Then the callback receives three incremental progress deliveries before the final tool result

  Scenario: Clearing the callback removes the registration for that session id
    Given a tool-progress callback is registered under a real session id S
    When the stream loop clears the callback for session id S
    And progress is emitted under session id S
    Then no callback is invoked for session id S

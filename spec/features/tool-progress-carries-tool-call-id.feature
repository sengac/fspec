@done
@tool-display
@cli
@rust
@tool-call
@streaming
@BUG-149
Feature: Live tool output not folded into TUI card: ToolProgress emitted with empty tool_call_id

  """
  Uses bcrypt... N/A. Architecture: emit-side change in stream_loop.rs threads active tool_call.id into the progress callback; match-side in fspec-tui chunk_processor.rs is unchanged (exact-id fold). Serial tool execution within a turn (tool_execution_in_progress flag) makes single active id unambiguous.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ToolProgress emitted during tool execution MUST carry the same tool_call_id as the ToolCall that started the tool
  #   2. The TUI folds a ToolProgress chunk into a ToolCall card only when the progress tool_call_id exactly matches the card's tool_call_id
  #   3. The active tool_call_id is tracked while a tool runs (set on ToolCall, cleared on ToolResult) so the progress callback can emit the correct id
  #   4. Session isolation is preserved: progress for one session is never folded into another session's card
  #   5. A stray ToolProgress emitted when no tool is active must not panic and must not corrupt an unrelated card
  #
  # EXAMPLES:
  #   1. A ToolCall with id 'tc-1' starts bash; the progress emitted for its output carries tool_call_id 'tc-1'
  #   2. A ToolCall card 'tc-1' exists; a ToolProgress with tool_call_id 'tc-1' folds its output into card tc-1's body while still streaming
  #   3. A ToolProgress with an empty tool_call_id matches no card and its output is not shown (regression guard for the old behavior)
  #   4. Card tc-1 exists for session A; a ToolProgress for session B (card tc-2) does not alter card tc-1
  #   5. The active-tool-call tracking: id is present between ToolCall and ToolResult and absent (cleared) after the result
  #
  # ========================================

  Background: User Story
    As a developer using the fspec-tui
    I want to see bash/tool output stream into the tool-call card line-by-line while the command runs
    So that I get live feedback instead of waiting until the command finishes

  Scenario: Progress with a matching tool_call_id folds into the streaming card
    Given the agent view scrollback contains a ToolCall card with tool_call_id "tc-1"
    When a ToolProgress with tool_call_id "tc-1" arrives while the command is running
    Then the card "tc-1" body shows the streamed output
    And the card "tc-1" is still marked as streaming

  Scenario: Progress with an empty tool_call_id matches no card and is dropped
    Given the agent view scrollback contains a ToolCall card with tool_call_id "tc-1"
    When a ToolProgress with an empty tool_call_id arrives
    Then the card "tc-1" body does not show that output

  Scenario: Progress for another session's card does not alter this card
    Given a ToolCall card with tool_call_id "tc-1" exists
    And a separate ToolCall card with tool_call_id "tc-2" exists
    When a ToolProgress with tool_call_id "tc-2" arrives
    Then the card "tc-1" body is unchanged

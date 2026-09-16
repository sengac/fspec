@done
@tui-component
@scrollback
@markdown
@agent-view
@RPC-432
Feature: Render markdown tables at each assistant-message finalization, not only at turn Done

  """
  format_markdown_tables (rust/fspec-tui/src/store/agent_view/markdown_tables.rs, RPC-370) moves from handle_done into flush_in_flight_drop_empty (rust/fspec-tui/src/store/agent_view/chunk_processor.rs), so every flush trigger (ToolCall, Error, Interrupted, UserInput) finalizes the in-flight assistant message with the box-drawing grid; handle_done reduces to the shared flush plus the thinking-slot clear. No new call site beyond the existing flush_triggers. Intentional divergence from the TS Ink chunkProcessor (RPC-091/RPC-370), which only formats at Done.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every non-empty in-flight AssistantText chunk MUST be run through format_markdown_tables at finalization, regardless of which chunk finalizes it (ToolCall, Error, Interrupted, UserInput, or Done) — grid rendering is a property of message completion, not of turn completion
  #   2. format_markdown_tables MUST run exactly once per assistant message — the flush trigger applies it when the in-flight chunk is non-empty, and StreamChunk::Done must not re-apply it to text that was already flushed as a message
  #
  # EXAMPLES:
  #   1. Assistant streams '| h1 | h2 |\n|---|---|\n| a | b |' then ToolCall(Bash) arrives → the in-flight assistant chunk is formatted at ToolCall time, so the scrollback shows the box-drawing grid BEFORE the tool-call card, with no re-formatting when Done later arrives
  #   2. A table in the turn's final message still renders as a grid — the single-message turn ('| a | b |\n|---|---|\n| 1 | 2 |' → Done) shows the box-drawing grid exactly as before
  #
  # ASSUMPTIONS:
  #   1. format_markdown_tables is idempotent: already-rendered box-drawing grids contain no pipe rows with a dash separator row, so a second pass leaves them unchanged — which is why routing every finalization through one format call is safe even though Done may arrive after a ToolCall already formatted the last message
  #
  # ========================================

  Background: User Story
    As a user reading AI responses in the Rust TUI AgentView
    I want to see every assistant message's markdown tables rendered as box-drawing grids at message finalization
    So that tables between tool calls align and read as cleanly as the turn-final message

  Scenario: ToolCall finalizes the intermediate assistant message with a box-drawing grid
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Text { text: "| h1 | h2 |\n|---|---|\n| a | b |" } then StreamChunk::ToolCall { tool_call: { id: "tc-1", name: "Bash", input: "{\"command\":\"ls\"}" } } for s-1
    Then the s-1 scrollback's first chunk source.text contains a box-drawing grid with top border "┌" and bottom border "└" instead of raw pipe rows
    And the first chunk's is_streaming flag is false
    And the s-1 scrollback ends with a ToolCall chunk for "tc-1" after the formatted assistant chunk


  Scenario: Error finalizes the in-flight assistant message with a box-drawing grid
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Text { text: "| a | b |\n|---|---|\n| 1 | 2 |" } then StreamChunk::Error { error: "rate limit exceeded" } for s-1
    Then the in-flight assistant chunk's source.text contains a box-drawing grid with top border "┌" and bottom border "└" instead of raw pipe rows
    And the in-flight assistant chunk's is_streaming flag is false


  Scenario: Interrupted finalizes the in-flight assistant message with a box-drawing grid
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Text { text: "| a | b |\n|---|---|\n| 1 | 2 |" } then StreamChunk::Interrupted for s-1
    Then the in-flight assistant chunk's source.text contains a box-drawing grid with top border "┌" and bottom border "└" instead of raw pipe rows
    And the in-flight assistant chunk's is_streaming flag is false


  Scenario: UserInput finalizes the in-flight assistant message with a box-drawing grid
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Text { text: "| a | b |\n|---|---|\n| 1 | 2 |" } then StreamChunk::UserInput { text: "next question" } for s-1
    Then the in-flight assistant chunk's source.text contains a box-drawing grid with top border "┌" and bottom border "└" instead of raw pipe rows
    And the in-flight assistant chunk's is_streaming flag is false


  Scenario: Done still formats the turn-final assistant message exactly once
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Text { text: "| a | b |\n|---|---|\n| 1 | 2 |" } then StreamChunk::Done for s-1
    Then the final chunk's source.text contains a box-drawing grid with top border "┌" and bottom border "└" instead of raw pipe rows
    And the final chunk's is_streaming flag is false


  Scenario: Done after a ToolCall leaves the already-formatted message unchanged
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Text { text: "| h1 | h2 |\n|---|---|\n| a | b |" } then StreamChunk::ToolCall { tool_call: { id: "tc-1", name: "Bash", input: "{\"command\":\"ls\"}" } } then StreamChunk::Text { text: "done listing" } then StreamChunk::Done for s-1
    Then the first chunk (intermediate assistant message) still contains a box-drawing grid with top border "┌" and bottom border "└"
    And the second assistant chunk (after the tool call) contains the plain text "done listing"


  Scenario: An intermediate assistant message without a table is unchanged at flush
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Text { text: "Let me check the board" } then StreamChunk::ToolCall { tool_call: { id: "tc-1", name: "Bash", input: "{\"command\":\"ls\"}" } } for s-1
    Then the first chunk's source.text equals "Let me check the board" byte-for-byte


  Scenario: Every assistant message in a multi-message turn renders its own grid
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Text { text: "| a | b |\n|---|---|\n| 1 | 2 |" } then StreamChunk::ToolCall { tool_call: { id: "tc-1", name: "Bash", input: "{\"command\":\"ls\"}" } } then StreamChunk::Text { text: "| x | y |\n|---|---|\n| 3 | 4 |" } then StreamChunk::Done for s-1
    Then both assistant chunks contain box-drawing grids with top border "┌" and bottom border "└"


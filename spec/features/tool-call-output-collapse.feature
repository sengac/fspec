@done
@rust
@ts-parity
@agent-view
@tui
@RPC-389
Feature: Tool Call Output Collapse

  """
  Fix site is wrap_source in chunk_wrap.rs: collapse/window happens at the ChunkSource->lines render layer; ChunkSource.text stays full. Mirrors AgentView.tsx formatCollapsedOutput (8) + createStreamingWindow (10)
  Constants COLLAPSED_LINES=8 and STREAMING_WINDOW_SIZE=10. Diff-style collapse (25/3-context, [R]-/[A]+) is OUT OF SCOPE — no inline diff renderer exists in the Rust port
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A settled tool-call card whose body is 8 or fewer lines shows the full body with no truncation indicator
  #   2. A settled tool-call card whose body exceeds 8 lines shows the LAST 8 body lines (end-pinned, RPC-399) followed by an indicator line '... +N lines (Enter to view full)' where N is the number of hidden body lines above the window
  #   3. While a tool-call card is still streaming, its body shows only the last 10 lines (tail window) with no indicator line
  #   4. The collapse/window applies only to ToolCall chunks; the '● ToolName(args)' header line is always kept and the line threshold counts hard newline-delimited body lines (pre-wrap), not other chunk kinds
  #   5. The full untruncated body is preserved in ChunkSource.text so the existing TurnContentModal (Enter in SELECT mode) still shows every line
  #
  # EXAMPLES:
  #   1. A settled tool card with a 5-line body renders all 5 lines and no '... +N lines' indicator
  #   2. A settled tool card with a 20-line body renders the last 8 body lines then '... +12 lines (Enter to view full)'
  #   3. A streaming tool card with a 25-line body renders only the last 10 body lines and no indicator
  #   4. A tool card that finishes streaming with a 25-line body stays end-pinned: it keeps the last body lines (last-8 window plus '... +17 lines (Enter to view full)') rather than jumping to the first lines
  #   5. Selecting a collapsed 20-line tool card and pressing Enter opens the TurnContentModal showing all 20 lines
  #
  # ========================================

  Background: User Story
    As a developer watching the agent TUI
    I want to see long tool output collapsed inline (first lines plus a 'more lines' hint, or a tail window while streaming) instead of the entire body dumped into the scrollback
    So that the chat stays readable and I can open the full output on demand

  Scenario: Settled tool card with a short body shows the full body
    Given a settled tool-call card whose body has 5 lines
    When the tool-call card is rendered into scrollback lines
    Then the rendered lines show all 5 body lines
    And no "... +N lines" indicator line is shown

  Scenario: Settled tool card with a long body collapses to the last 8 lines
    Given a settled tool-call card whose body has 20 lines
    When the tool-call card is rendered into scrollback lines
    Then the rendered lines show the last 8 body lines
    And the next rendered line is "... +12 lines (Enter to view full)"

  Scenario: Streaming tool card shows only the last 10 lines
    Given a streaming tool-call card whose body has 25 lines
    When the tool-call card is rendered into scrollback lines
    Then the rendered lines show only the last 10 body lines
    And no "... +N lines" indicator line is shown

  Scenario: A finished stream stays pinned to the end of output
    Given a streaming tool-call card whose body has 25 lines
    When the tool-call card finishes streaming
    And the tool-call card is rendered into scrollback lines
    Then the rendered lines show the last 8 body lines
    And the next rendered line is "... +17 lines (Enter to view full)"

  Scenario: The full body is preserved for the content modal
    Given a settled tool-call card whose body has 20 lines
    When the full text for the card's turn is requested for the TurnContentModal
    Then the returned text contains all 20 body lines

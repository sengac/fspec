@done
@agent-view
@tui
@ts-parity
@scrollback
@rust
@RPC-401
Feature: AgentView message line-spacing parity — missing per-message separator gutter
  """
  TS parity anchor: src/tui/utils/conversationUtils.ts wrapMessageToLines lines 117-127 append a trailing ConversationLine{content:' ', isSeparator:true} per message when addSeparator defaults true (called without override at AgentView.tsx:4889)
  Implement by appending one empty Line::from("") at the end of wrap_source in codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs. This is the single wrap entry point used by push_source, insert_source_at and rewrap_chunk, so the separator automatically flows into total_visual_rows, resize rewrap, and paint_chunk_rows
  wrap_source has early returns for Thinking-then-ToolCall (line 79-82) and diff (line 150-158) and the default path (line 104). Each return site must append the trailing blank line so all ChunkKind variants get it uniformly
  The TurnContentModal must NOT show the trailing separator. It sources full text via full_text_for_seq / ChunkSource.text (scrollback_select.rs:113), NOT the cached lines, so the wrap-level separator does not leak into the modal — verify this holds.
  Arrow-bar parity: paint_selection_arrow_bars (scrollback_arrows.rs:97-121) currently paints ▼ on fy-1 (row above first content row) and ▲ on ly+1 (row below last content row). With a trailing blank separator now occupying ly+1, the ▲ naturally lands on the blank gutter. The ▼ at fy-1 lands on the PREVIOUS chunk's trailing separator. scroll_selected_into_view already reserves +/-1 rows (scrollback_select.rs:213-214) so this stays correct — verify with a test.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every rendered chunk (message) emits exactly one trailing blank separator line after its content, matching the TS wrapMessageToLines addSeparator=true default
  #   2. The separator line is appended in wrap_source so it participates in total_visual_rows accounting, rewrap-on-resize, and paint_chunk_rows painting
  #   3. The separator applies uniformly to every ChunkKind: UserInput, AssistantText, Thinking, ToolCall (diff and non-diff), Error, Interrupted, Notification, Incoming
  #   4. The separator line is empty (renders as a blank row) and carries no prefix, marker, or color styling
  #   5. In turn-select (Item) mode the selection arrow-bars occupy the blank separator gutter row above/below the selected chunk rather than overwriting a content row
  #
  # EXAMPLES:
  #   1. A single user message 'hello' wraps to a 'You: hello' line followed by one blank separator line (2 total lines)
  #   2. Two consecutive assistant messages render with exactly one blank line between their content blocks
  #   3. A tool-call chunk with a collapsed body ends with the '... +N lines' indicator followed by one blank separator line
  #   4. total_visual_rows for three single-line messages equals 6 (3 content + 3 separators)
  #   5. A diff tool-call chunk keeps its typed diff rows and appends one trailing blank separator line
  #   6. In Item mode, selecting a middle turn paints the ▼ bar on the blank separator row above it and the ▲ bar on the blank separator row below it, without hiding any content
  #
  # ========================================
  Background: User Story
    As a user of the Rust ratatui AgentView TUI
    I want to see one blank line between each message in the scrollback
    So that the conversation is as readable and visually grouped as the TypeScript reference TUI

  Scenario: A single user message ends with one blank separator line
    Given a user message with text "hello"
    When the message is wrapped for the scrollback
    Then the wrapped output has 2 lines
    And the first line is "You: hello"
    And the last line is blank

  Scenario: Two consecutive assistant messages are separated by one blank line
    Given an assistant message with text "first"
    And an assistant message with text "second"
    When both messages are wrapped and painted into the scrollback
    Then exactly one blank line appears between the two content blocks

  Scenario: A collapsed tool-call chunk ends with a blank separator line after its indicator
    Given a settled tool-call chunk whose body exceeds the collapse threshold
    When the tool-call chunk is wrapped for the scrollback
    Then the second to last line is the "... +N lines" indicator
    And the last line is blank

  Scenario: Total visual rows include one separator per message
    Given three single-line messages in the scrollback
    When the total visual rows are computed
    Then the total visual rows equal 6

  Scenario: A diff tool-call chunk keeps its diff rows and ends with a blank separator line
    Given a diff tool-call chunk with typed diff rows
    When the diff tool-call chunk is wrapped for the scrollback
    Then the typed diff rows are preserved
    And the last line is blank

  Scenario: In Item mode arrow bars occupy the gutter rows without hiding content
    Given three single-line messages in the scrollback in Item mode
    And the middle turn is selected
    When the scrollback is painted
    Then the down arrow bar is painted on the blank separator row above the selected turn
    And the up arrow bar is painted on the blank separator row below the selected turn
    And the selected turn's content row is still visible

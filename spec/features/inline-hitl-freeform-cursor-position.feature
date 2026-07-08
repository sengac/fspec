@done
@input
@agent-view
@rust
@RPC-412
Feature: Inline HITL freeform cursor renders one row above the input line
  """
  render_hitl_prompt already returns Some(header_offset) in freeform mode; capture it in paint_input_area into a new AgentView field (e.g. last_hitl_input_offset: Option<u16>), reset each paint. cursor_position() anchors at last_input_area.y + offset when present. Column math and viewport clamp in hardware_cursor_in are preserved (offset applied to the region y before clamping so the cursor can reach the true input row).
  Parity anchors: TS src/tui/components/MultiLineInput.tsx renders the > input as a flex child below the prompt header so Ink places the inverse cursor on the input line automatically; the Rust raw set_cursor_position must reproduce that. tui-textarea (widget.rs:168) stores the painted viewport during render and derives the cursor from where the input was actually painted — apply the same principle here (anchor to the painted input row, not the region top).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. In HITL freeform mode the hardware cursor row must equal the row where the shared composer input line is painted (header rows + optional hint row below the input area top), not the input area top
  #   2. The cursor row offset must be the ACTUAL header offset returned by render_hitl_prompt (which grows when the header wraps to multiple rows or the empty-submit hint row is shown), never a hardcoded plus one
  #   3. The cursor column (X) math is unchanged and the cursor stays clamped inside the input viewport after the row offset is applied
  #   4. Options-mode HITL prompts and RPC-406 pause prompts still report NO hardware cursor (the cursor gate is unchanged) and are unaffected by the offset
  #   5. Normal (non-prompt) composer input is unaffected: its cursor still anchors at the input area top because no header is painted above the input line
  #
  # EXAMPLES:
  #   1. A single-row-header freeform question with no hint: after painting, the reported cursor row is input_area.y + 1 (one header row), the same row as the > input line
  #   2. A freeform question with the empty-submit hint shown: the cursor row is input_area.y + 2 (header row + yellow hint row), matching the pushed-down > input line
  #   3. The Other sub-mode of an options question (freeform active): the cursor sits on the > input line below the header, not on the header
  #   4. A long question wrapped to a 2-row header at a narrow width: the cursor row is input_area.y + 2, proving the offset tracks wrapped header rows and is not a constant plus one
  #   5. Regression guard: an options-mode HITL prompt reports no hardware cursor at all (cursor gate returns false), so no offset is applied
  #   6. Normal composer with no prompt: the cursor row equals input_area.y (no header offset added)
  #
  # ========================================
  Background: User Story
    As a TUI user answering a HITL request with freeform text
    I want to see the terminal cursor sit on the same line as the characters I type
    So that I can tell where my input is going and trust the freeform prompt

  Scenario: Freeform question with a single-row header places the cursor on the input line
    Given a focused session with an active pure-freeform HITL question whose header fits on one row
    And no empty-submit hint is showing
    When the input area is painted and the hardware cursor position is queried
    Then the reported cursor row is one row below the input area top
    And that row is the same row where the "> " composer input line is painted

  Scenario: The empty-submit hint pushes the cursor down onto the input line
    Given a focused session with an active pure-freeform HITL question whose header fits on one row
    And the empty-submit hint is showing
    When the input area is painted and the hardware cursor position is queried
    Then the reported cursor row is two rows below the input area top
    And that row is the same row where the "> " composer input line is painted

  Scenario: The Other sub-mode places the cursor on the input line below the header
    Given a focused session with an active options HITL question in the "Other..." freeform sub-mode
    When the input area is painted and the hardware cursor position is queried
    Then the reported cursor row is below the header on the "> " composer input line
    And the cursor is not on the header row

  Scenario: A header wrapped to two rows offsets the cursor by two rows
    Given a focused session with an active pure-freeform HITL question whose header wraps to two rows at a narrow width
    And no empty-submit hint is showing
    When the input area is painted and the hardware cursor position is queried
    Then the reported cursor row is two rows below the input area top
    And the offset tracks the wrapped header height rather than a fixed single row

  Scenario: An options-mode HITL prompt shows no hardware cursor
    Given a focused session with an active options HITL question that is not in the freeform sub-mode
    When the cursor visibility is queried for the input area
    Then no hardware cursor is reported
    And no header offset is applied

  Scenario: The normal composer anchors the cursor at the input area top
    Given a focused session with no active HITL prompt and no pause prompt
    When the input area is painted and the hardware cursor position is queried
    Then the reported cursor row equals the input area top
    And no header offset is added

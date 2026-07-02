@ui-enhancement
@ui
@input
@agent-view
@tui
@rust
@done
@RPC-405
Feature: MultiLineInput lacks soft-wrap: input area never grows when text flows past the terminal width
  """
  TextArea (tui-textarea) stays the STATE engine only — buffer, cursor, editing ops, Enter/paste routing and gates are unchanged. The render layer is replaced: a new pure wrap-geometry module (multiline_wrap.rs) segments logical lines into visual rows by unicode display width (never splitting wide chars; empty lines = one visual row). The widget owns a visual-row viewport with the tui-textarea next_scroll_top cursor-follow algorithm transplanted into visual-row space (clamped to content). The AgentView layout computes the input height via visible_rows_for_width(body_width) using the SAME body width the renderer paints with (area - 2x1 padding - 2-col prompt), clamped to [1, 6] visual rows. A logical-to-visual cursor mapping (visual row + display column) is exposed for RPC-404 hardware-cursor positioning.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Wrap segmentation uses unicode display width (never splits a wide char) and preserves empty logical lines as one visual row each
  #   2. Input area height equals total wrapped visual rows clamped to [1, 6], computed from the same body width the renderer uses
  #   3. When total visual rows exceed the cap, a visual-row viewport scrolls to keep the cursor row visible (next_scroll_top follow algorithm) and clamps to the content
  #   4. The renderer paints wrapped segments only (no horizontal scrolling); the head of the text is never pushed off-screen
  #   5. TextArea remains the buffer/state engine; value(), set_value(), editing, Enter/paste routing and gates are unchanged
  #   6. MultiLineInput exposes a logical-to-visual cursor mapping (visual row + display column) for the app cursor positioning (consumed by RPC-404)
  #
  # EXAMPLES:
  #   1. Typing an 84-char line into a 60-col terminal grows the input to 2 rows and both word01 (head) and word12 (tail) are visible (fixes zz_repro wrap case)
  #   2. Recalling a 3-line history entry shows all three lines in a 3-row input; the scrollback above shrinks by 2 rows
  #   3. A buffer wrapping to 9 visual rows shows a 6-row window that follows the cursor: cursor at end shows the last 6 rows; moving the cursor to the top scrolls the window up
  #   4. Deleting text shrinks the input back down (2 rows to 1) and the scrollback regains the row; empty buffer shows the 1-row placeholder
  #   5. A line of wide CJK characters wraps without splitting any character; a mixed emoji line wraps at the correct display-width boundary
  #   6. Pressing Enter mid-wrap still submits the entire buffer text unchanged (wrapping is visual only)
  #
  # ASSUMPTIONS:
  #   1. Cap counts VISUAL rows (not logical lines like the TS maxVisibleLines=5) because the ratatui layout needs a hard terminal-row budget; cap value stays 6
  #   2. Tab-stop expansion is out of scope: '\t' contributes its unicode-width fallback; must not panic. TS reference never handles tabs either
  #
  # ========================================
  Background: User Story
    As a TUI user typing in the agent input
    I want to have the input box grow vertically as my text wraps or spans multiple lines, with the message area shrinking above it
    So that I can see everything I am typing (head and tail) exactly like the TypeScript AgentView input

  Scenario: Typing past the right edge grows the input to two rows with head and tail visible
    Given the agent view is rendered on a 60x12 terminal
    And the input buffer contains the single 83-character line "word01 word02 word03 word04 word05 word06 word07 word08 word09 word10 word11 word12"
    When the agent view renders a frame
    Then the input area is 2 terminal rows tall
    And the first input row shows text starting with "word01"
    And the second input row shows text ending with "word12"
    And no input row shows more than 56 columns of buffer text

  Scenario: Recalled multi-line history entry renders fully and shrinks the scrollback
    Given the agent view is rendered on a 60x12 terminal with an empty input
    And the scrollback area is 9 terminal rows tall
    When the user recalls a history entry consisting of the three lines "alpha", "bravo" and "charlie"
    Then the input area is 3 terminal rows tall
    And the rows "alpha", "bravo" and "charlie" are each visible on their own input row
    And the scrollback area is 7 terminal rows tall

  Scenario: Buffer wrapping past the cap shows the last six visual rows when the cursor is at the end
    Given the agent view is rendered on a 60x12 terminal
    And the input buffer contains nine logical lines "row01" through "row09"
    And the cursor is at the end of "row09"
    When the agent view renders a frame
    Then the input area is 6 terminal rows tall
    And the rows "row04" through "row09" are visible in the input area
    And the row "row01" is not visible

  Scenario: Moving the cursor to the top scrolls the six-row window up
    Given the agent view is rendered on a 60x12 terminal
    And the input buffer contains nine logical lines "row01" through "row09"
    And the cursor is at the end of "row09"
    When the user presses the Up arrow key 8 times
    Then the input area is 6 terminal rows tall
    And the rows "row01" through "row06" are visible in the input area
    And the row "row09" is not visible

  Scenario: Deleting text shrinks the input back down and the scrollback regains the row
    Given the agent view is rendered on a 60x12 terminal
    And the input buffer contains the single 83-character line "word01 word02 word03 word04 word05 word06 word07 word08 word09 word10 word11 word12"
    And the input area is 2 terminal rows tall
    When the user deletes characters until the buffer is the 55-character line "word01 word02 word03 word04 word05 word06 word07 word08"
    Then the input area is 1 terminal row tall
    And the scrollback area is 9 terminal rows tall

  Scenario: Empty buffer shows the one-row placeholder
    Given the agent view is rendered on a 60x12 terminal
    And the input buffer is empty
    When the agent view renders a frame
    Then the input area is 1 terminal row tall
    And the input row shows the placeholder hint starting with "Type a message..."

  Scenario: Wide CJK characters wrap without being split
    Given the agent view is rendered on a 60x12 terminal
    And the input buffer contains a single line of 30 "漢" characters with a display width of 60 columns
    When the agent view renders a frame
    Then the input area is 2 terminal rows tall
    And the first input row shows exactly 28 "漢" characters filling 56 columns
    And the second input row shows the remaining 2 "漢" characters
    And no "漢" character is split across two rows

  Scenario: Emoji wraps at the correct display-width boundary
    Given the agent view is rendered on a 60x12 terminal
    And the input buffer contains a single line of 55 "a" characters followed by "😀" with a display width of 57 columns
    When the agent view renders a frame
    Then the input area is 2 terminal rows tall
    And the first input row shows exactly the 55 "a" characters
    And the second input row starts with "😀"

  Scenario: Pressing Enter mid-wrap submits the entire buffer unchanged
    Given the agent view is rendered on a 60x12 terminal
    And the input buffer contains the single 83-character line "word01 word02 word03 word04 word05 word06 word07 word08 word09 word10 word11 word12"
    And the cursor has been moved 40 characters to the left so it sits on the first visual row
    When the user presses Enter
    Then the submitted value is the full 83-character line "word01 word02 word03 word04 word05 word06 word07 word08 word09 word10 word11 word12"
    And the submitted value contains no newline characters

  Scenario: Input height derives from the exact renderer body width
    Given the agent view is rendered on a 60x12 terminal
    And the input body width is 56 columns after 1 column of padding on each side and the 2-column "> " prompt
    And the input buffer contains a single line of 57 "x" characters
    When the agent view renders a frame
    Then the input area is 2 terminal rows tall
    And the first input row shows exactly 56 "x" characters
    And the second input row shows exactly 1 "x" character

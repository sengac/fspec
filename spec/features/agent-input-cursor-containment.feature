@ui-enhancement
@ui
@input
@agent-view
@tui
@rust
@done
@RPC-404
Feature: Hardware cursor escapes input viewport when buffer exceeds 6 visible rows
  """
  Maps the logical cursor through RPC-405's wrap geometry: (vrow, vcol) = MultiLineInput::cursor_visual(body_width) where body_width = input_area.width - 2x1 padding - 2-col '> ' prompt (MUST match the render path in views/agent.rs), then y = area.y + (vrow - scroll_top()) using the visual-row viewport top after cursor-follow, and x = area.x + 1 (pad) + 2 (prompt) + vcol where vcol is a DISPLAY column (unicode width, not char index). Defensive clamp: x into [area.x, area.x + width - 1] and y into [area.y, area.y + height - 1] so the hardware cursor can never leave the input rect. cursor_position() still returns None when no input area has been recorded; cursor-visibility gating stays in is_cursor_visible.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. cursor_position() maps the logical cursor through the RPC-405 wrap geometry (cursor_visual) and subtracts the viewport scroll_top
  #   2. The returned (x, y) is always inside the input area rect for any buffer, cursor and area combination (defensive clamp)
  #   3. The x coordinate uses the display column (unicode width), not the char index
  #
  # EXAMPLES:
  #   1. With a 10-line buffer and the 6-row cap, cursor on the last line sits on the LAST row of the input area, not rows below the terminal bottom
  #   2. After scrolling the window up (cursor moved to the first line), the cursor sits on the FIRST row of the input area
  #   3. A single logical line wrapped to 3 visual rows with the cursor mid-line places the cursor on the matching wrapped row and column
  #   4. A CJK line places the cursor at the display-width column (two cells per ideograph), not the char index
  #   5. Across a grid of buffer/cursor cases the cursor position always lies inside the input rect (containment property)
  #
  # ========================================
  Background: User Story
    As a TUI user editing multi-line or wrapped text in the agent input
    I want to see the hardware cursor exactly on the cell I am editing, always inside the input box
    So that I never lose track of my edit position and the cursor never lands outside the input area or off-screen

  Scenario: Cursor on the last line of a 10-line buffer sits on the last input-area row
    Given the agent view is rendered on a 60x12 terminal
    And the input buffer contains ten logical lines "line01" through "line10"
    And the cursor is at the end of "line10"
    When the agent view renders a frame and cursor_position() is queried
    Then the input area is 6 terminal rows tall occupying rows 6 through 11
    And the cursor y coordinate is 11, the last row of the input area
    And the cursor x coordinate is 9, the body start column 3 plus the 6-character display column
    And the cursor is not below the terminal bottom

  Scenario: Scrolling the window up with the cursor on the first line pins the cursor to the first input-area row
    Given the agent view is rendered on a 60x12 terminal
    And the input buffer contains ten logical lines "line01" through "line10"
    And the cursor is at the end of "line10"
    When the user presses the Up arrow key 9 times and cursor_position() is queried after a render
    Then the cursor is on the first logical line
    And the cursor y coordinate is 6, the first row of the input area
    And the cursor x coordinate is 9, the body start column 3 plus the 6-character display column

  Scenario: Cursor mid-way through a line wrapped to three visual rows lands on the matching wrapped row and column
    Given the agent view is rendered on a 60x12 terminal
    And the input buffer contains a single 120-character line of "x" characters wrapping to visual rows of 56, 56 and 8 columns
    And the cursor has been moved 40 characters to the left from the end so it sits at char index 80
    When the agent view renders a frame and cursor_position() is queried
    Then the input area is 3 terminal rows tall occupying rows 9 through 11
    And the cursor y coordinate is 10, the second visual row of the wrapped line
    And the cursor x coordinate is 27, the body start column 3 plus display column 24 within the second segment

  Scenario: CJK line places the cursor at the display-width column not the char index
    Given the agent view is rendered on a 60x12 terminal
    And the input buffer contains the single line "漢漢漢漢漢"
    And the cursor has been moved 2 characters to the left from the end so it sits after the 3rd ideograph at char index 3
    When the agent view renders a frame and cursor_position() is queried
    Then the cursor x coordinate is 9, the body start column 3 plus display column 6 for three double-width ideographs
    And the cursor x coordinate is not 6, which would be the body start column plus the char index 3
    And the cursor y coordinate is 11, the single input row

  Scenario: Cursor position always lies inside the input area rect across a grid of buffer and cursor cases
    Given the agent view is rendered on a 60x12 terminal
    And a grid of input buffers: an empty buffer, a single short line, a single 120-character wrapped line, and ten logical lines
    And for each buffer the cursor is placed at the line start, the middle and the end via Home, arrow keys and End
    When the agent view renders a frame and cursor_position() is queried for every grid case
    Then every returned cursor position lies inside the input area rect recorded by the render
    And no returned cursor y coordinate exceeds row 11, the terminal bottom

@done
@integration
@rust
@mouse-events
@clipboard
@agent-view
@text-selection
@tui
@COPY-007
Feature: Wire selection + copy into the AgentView input composer
  """
  Wiring point: views/agent/multiline_input.rs currently drops Event::Mouse via `_ => Ignored`. Add a handle_mouse path on MultiLineInput (or route from AgentView dispatch.rs Event::Mouse before it reaches input) that converts mouse (col,row) into composer visual (row,col) by subtracting the input area origin + PROMPT_WIDTH + INPUT_PAD_X, then feeds the SelectionRecognizer (COPY-003).
  State: add selection: Option<Selection> to MultiLineInput (co-located with buffer + scroll_top). Recognizer + Osc52Clipboard live on AgentView/SessionContext (need action bus + stdout), same as COPY-006. Reuse COPY-002 Selection + COPY-003 recognizer + COPY-001 writer unchanged.
  Text reconstruction: adapt COPY-004's row-span reader to read from multiline_wrap::wrap_lines(value, input_body_width) windowed by scroll_top, excluding the prompt/pad columns. Highlight: multiline_input_render.rs::render paints REVERSED cells (COPY-005 overlay) for the visible selection rows before/around the hardware cursor paint.
  Clear triggers: is_edit_keystroke (multiline_input_enter.rs) and any set_value/reset clear selection; scroll_top change clears; Esc precedence added in dispatch.rs BEFORE the composer's own Esc/submit handling. Testing: (1) unit MultiLineInput handle_mouse Down+Drag+Up -> Begin/Extend/Commit and injected Vec<u8> Osc52 receives prompt-free text; (2) selection cleared on edit keystroke/scroll/Esc; (3) render buffer shows REVERSED cells excluding the `> ` columns.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Event::Mouse over the input rect is fed to the gesture recognizer (COPY-003) instead of being dropped by the composer's `_ => Ignored` arm
  #   2. Selection is expressed over the composer's WRAPPED visual rows (multiline_wrap::wrap_lines geometry), not raw logical lines, so highlight and copied text match what is on screen
  #   3. The `> ` prompt prefix (PROMPT_WIDTH) and the left INPUT_PAD_X padding are excluded from the copied text and highlight
  #   4. On Commit the reconstructed selected text is written via the OSC 52 writer (COPY-001) and the highlight is retained; mouse capture stays enabled throughout
  #   5. Editing keystrokes (typing/backspace/cursor moves) and a change in the input scroll offset clear the composer selection; Esc clears an active composer selection before any other Esc handling
  #   6. A quick click (no drag, short hold) creates no selection and writes nothing to the clipboard
  #
  # EXAMPLES:
  #   1. User types a multi-line draft, drags across the second wrapped row, releases, and that row's text (no `> ` prompt) is on the clipboard and stays highlighted
  #   2. User long-presses a line in the composer for ~0.5s then releases; that line is selected and copied
  #   3. User has a composer selection then types a character; the selection clears and the character is inserted normally
  #   4. User has a composer selection then presses Esc; the highlight disappears, nothing is copied on Esc, and the input is not submitted/cleared
  #   5. User quickly clicks in the composer to move the cursor; nothing is selected or copied
  #
  # ========================================
  Background: User Story
    As a user typing in the composer
    I want to select and copy text from the input box with the mouse
    So that I can reuse what I typed (or copy a draft) without mouse capture stealing my selection

  Scenario: Dragging across a wrapped composer row copies its text without the prompt
    Given a composer holding a multi-line draft with mouse capture enabled
    When I drag across the second wrapped row of the input and release
    Then that row's text without the "> " prompt is written to the clipboard
    And the composer selection stays highlighted

  Scenario: Long-pressing a composer line selects and copies it
    Given a composer holding a multi-line draft with mouse capture enabled
    When I press and hold on a composer line for about half a second and release
    Then that line becomes selected and its text is written to the clipboard

  Scenario: Typing while a selection is active clears it and inserts the character
    Given a composer with an active text selection
    When I type a character
    Then the composer selection is cleared
    And the character is inserted into the input normally

  Scenario: Esc clears an active composer selection without copying or submitting
    Given a composer with an active text selection
    When I press Esc
    Then the composer highlight disappears
    And nothing is written to the clipboard by the Esc press
    And the input is not submitted or cleared

  Scenario: A quick click in the composer does not select or copy
    Given a composer holding a multi-line draft with mouse capture enabled
    When I quickly click in the composer to move the cursor
    Then nothing is selected and nothing is written to the clipboard

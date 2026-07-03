@done
@integration
@rust
@mouse-events
@clipboard
@board
@text-selection
@tui
@COPY-009
Feature: Wire selection + copy into the BoardView work-unit details strip

  """
  Wiring point: views/board/render.rs render_with_store already caches last_content_area etc. from `split`. Add a `last_details_area` (Cell<Option<Rect>>) on BoardView set to borders::inner_rect(split[3]) — the details_strip inner rect. In views/board/mouse.rs handle_mouse, hit-test that rect FIRST; if the event lands there, feed the MouseEvent to a SelectionRecognizer (COPY-003), converting (col,row) to strip (row,col) by subtracting the rect origin. Otherwise fall through to the existing wheel/click logic unchanged.
  State: recognizer + Osc52Clipboard live on BoardView (need action bus + stdout, like MouseTrackingToggle wiring). Selection: Option<Selection> (COPY-002) held on BoardView, cleared when the selected work unit changes (SetFocusedColumn/SelectIndexInFocused that changes selected_work_unit()) or on Esc. Reuse COPY-001/002/003 unchanged.
  Text reconstruction: the strip has no scrollbar, so the reader reproduces the exact on-screen rows produced by details_strip::render (id:title via truncate_to, description via wrap_to_two_lines, attachments/metadata lines) for the selected row-span, excluding the two vertical border columns painted by paint_side_borders. This is simpler than COPY-004's scrollback reader (fixed 5 rows, no scroll). Highlight: paint REVERSED cells (COPY-005) over the selected strip rows during details_strip::render or as an overlay after it in render_with_store.
  Testing strategy: (1) unit — BoardView handle_mouse Down+Drag+Up inside the cached details rect produces Begin/Extend/Commit and injected Vec<u8> Osc52Clipboard receives the exact visible strip text (border-free); a Down outside the rect still yields the existing SetFocusedColumn/SelectIndexInFocused. (2) clear — changing the selected work unit or Esc clears the selection. (3) render — buffer shows REVERSED cells over the selected strip rows and never over the │ border columns. Prefer real BoardView + BoardStore + injected writer over mocks.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. During render_with_store the details-strip inner rect (split[3] inner) is cached (like last_content_area) so mouse events can be hit-tested against it
  #   2. Event::Mouse landing inside the cached details-strip rect is fed to the gesture recognizer (COPY-003) before the existing board wheel/click handling in views/board/mouse.rs handle_mouse
  #   3. Selection is over the strip's on-screen rows (the id: title row, the wrapped/ellipsized cyan description rows, attachments, metadata); copied text is the visible text of those rows, matching wrap/truncation exactly
  #   4. The left/right vertical border columns are excluded from the copied text and highlight
  #   5. On Commit the reconstructed selected text is written via the OSC 52 writer (COPY-001) and the highlight is retained; mouse capture stays enabled throughout
  #   6. Existing board behaviour is preserved: wheel over the column content still scrolls/selects rows and clicks in the column grid/headers still focus/select; a wheel or click OUTSIDE the details strip does not start a selection
  #   7. Selecting a different work unit (which changes the strip content), or pressing Esc, clears an active strip selection; a quick click in the strip does not create a selection or copy
  #
  # EXAMPLES:
  #   1. User selects a work unit, drags across the `ID: title` row, releases; `RPC-014: Board grid` (the visible text) lands on the clipboard and stays highlighted
  #   2. User drags across both wrapped cyan description rows; the two visible (ellipsized) lines are copied exactly as shown
  #   3. User drags a full-width description line to the strip edge; the copied text excludes the │ side border glyph
  #   4. User has a strip selection then scrolls a column or clicks another card; the strip selection clears and the board behaves normally
  #   5. User has a strip selection then presses Esc; the highlight clears and no copy occurs on Esc
  #   6. User quickly clicks inside the details strip; nothing is selected or copied and the click is otherwise inert (strip is not a selectable column)
  #
  # ========================================

  Background: User Story
    As a user browsing the board
    I want to select and copy the work-unit id, title, or description text from the details strip
    So that I can paste a story's id or description elsewhere without mouse capture blocking the selection

  Scenario: Dragging across the id and title row copies its visible text
    Given a board with a work unit selected and its details strip visible
    When I drag across the id and title row of the details strip and release
    Then the visible id and title text is written to the clipboard
    And the strip selection stays highlighted

  Scenario: Dragging across the wrapped description rows copies them as shown
    Given a board with a work unit selected and its details strip visible
    When I drag across both wrapped description rows and release
    Then the two visible description lines are written to the clipboard exactly as shown

  Scenario: Copying a full-width description line excludes the side border glyph
    Given a board with a work unit whose description fills the strip width
    When I drag a full-width description line to the strip edge and release
    Then the clipboard text excludes the side border glyph

  Scenario: Changing the selected work unit clears an active strip selection
    Given a board with an active details-strip selection
    When I scroll a column or click another card
    Then the strip selection is cleared
    And the board behaves normally

  Scenario: Esc clears an active strip selection without copying
    Given a board with an active details-strip selection
    When I press Esc
    Then the strip highlight clears
    And nothing is written to the clipboard by the Esc press

  Scenario: A quick click in the details strip does not select or copy
    Given a board with a work unit selected and its details strip visible
    When I quickly click inside the details strip
    Then nothing is selected and nothing is written to the clipboard
